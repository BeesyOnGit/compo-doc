use std::collections::HashSet;

use anyhow::{Result, bail};
use swc_common::SourceMap;
use swc_common::sync::Lrc;
use swc_ecma_ast::*;
use swc_ecma_codegen::{
    Emitter, Node,
    text_writer::{JsWriter, WriteJs},
};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};
use swc_ecma_visit::Visit;

pub struct TypeExtractor {
    pub target_type: String,
    pub found_props: Option<Vec<(String, String)>>,
}

impl TypeExtractor {
    pub fn new(target_type: &str) -> Self {
        Self {
            target_type: target_type.to_string(),
            found_props: None,
        }
    }

    pub fn extract_from_str(&mut self, code: &str) -> Result<String> {
        let cm = Lrc::new(SourceMap::default());
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            EsVersion::Es2020,
            StringInput::new(code, Default::default(), Default::default()),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser
            .parse_module()
            .map_err(|e| println!("{:?}", e))
            .unwrap();

        self.visit_module(&module);

        match &self.found_props {
            Some(props) => {
                let formatted = props
                    .iter()
                    .map(|(name, ty)| format!("  {}: {};", name, ty))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(format!("{{\n{}\n}}", formatted))
            }
            None => bail!("Type/Interface '{}' not found", self.target_type),
        }
    }
}

impl Visit for TypeExtractor {
    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        if n.id.sym == self.target_type {
            if let TsType::TsTypeLit(type_lit) = &*n.type_ann {
                let mut props = Vec::new();
                let cm = Lrc::new(SourceMap::default());

                for member in &type_lit.members {
                    if let TsTypeElement::TsPropertySignature(prop) = member {
                        let prop_name = prop.key.as_ident().map(|i| i.sym.to_string()).unwrap();

                        // First create the buffer
                        let mut type_buf = Vec::new();

                        // Use nested scope to contain the emitter
                        {
                            let writer = JsWriter::new(cm.clone(), "\n", &mut type_buf, None);
                            let mut emitter = Emitter {
                                cfg: swc_ecma_codegen::Config::default(),
                                cm: cm.clone(),
                                comments: None,
                                wr: Box::new(writer) as Box<dyn WriteJs>,
                            };

                            if let Some(type_ann) = &prop.type_ann {
                                type_ann.type_ann.emit_with(&mut emitter).unwrap();
                            } else {
                                emitter.wr.write_str("any").unwrap();
                            }
                        } // Emitter and writer are dropped here

                        // Now we can safely consume type_buf
                        let prop_type = String::from_utf8(type_buf).unwrap();
                        props.push((prop_name, prop_type.trim().to_string()));
                    }
                }

                self.found_props = Some(props);
            }
        }
    }

    fn visit_ts_interface_decl(&mut self, n: &TsInterfaceDecl) {
        if n.id.sym == self.target_type {
            let mut props = Vec::new();
            let cm = Lrc::new(SourceMap::default());

            for member in &n.body.body {
                if let TsTypeElement::TsPropertySignature(prop) = member {
                    let prop_name = prop.key.as_ident().map(|i| i.sym.to_string()).unwrap();

                    let mut type_buf = Vec::new();
                    {
                        let writer = JsWriter::new(cm.clone(), "\n", &mut type_buf, None);
                        let mut emitter = Emitter {
                            cfg: swc_ecma_codegen::Config::default(),
                            cm: cm.clone(),
                            comments: None,
                            wr: Box::new(writer) as Box<dyn WriteJs>,
                        };

                        if let Some(type_ann) = &prop.type_ann {
                            type_ann.type_ann.emit_with(&mut emitter).unwrap();
                        } else {
                            emitter.wr.write_str("any").unwrap();
                        }
                    }

                    let prop_type = String::from_utf8(type_buf).unwrap();
                    props.push((prop_name, prop_type.trim().to_string()));
                }
            }

            self.found_props = Some(props);
        }
    }
}

use swc_ecma_visit::VisitWith;

/// Try to discover which locally-declared type or interface
/// is actually *used* inside the component code.
///
/// `Ok(Some(name))`  – first match found  
/// `Ok(None)`        – nothing referenced  
/// `Err(_)`          – syntax error while parsing
pub fn find_used_type(code: &str) -> Result<Option<String>> {
    // 1. Parse ----------------------------------------------------------------
    let cm: Lrc<SourceMap> = Default::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            ..Default::default()
        }),
        EsVersion::Es2022,
        StringInput::new(code, Default::default(), Default::default()),
        None,
    );
    let module = Parser::new_from(lexer)
        .parse_module()
        .map_err(|e| println!("{:?}", e))
        .unwrap();

    // 2. Walk the AST ----------------------------------------------------------
    let mut finder = Finder::default();
    module.visit_with(&mut finder);
    Ok(finder.used)
}

#[derive(Default)]
pub struct Finder {
    defined: HashSet<String>,
    used: Option<String>,
}

impl Visit for Finder {
    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        self.defined.insert(n.id.sym.to_string());
        n.visit_children_with(self);
    }

    fn visit_ts_interface_decl(&mut self, n: &TsInterfaceDecl) {
        self.defined.insert(n.id.sym.to_string());
        n.visit_children_with(self);
    }

    fn visit_ts_type_ref(&mut self, n: &TsTypeRef) {
        if self.used.is_none() {
            if let TsEntityName::Ident(id) = &n.type_name {
                let name = id.sym.to_string();
                if self.defined.contains(&name) {
                    self.used = Some(name);
                    return; // short-circuit - we found one
                }
            }
        }
        n.visit_children_with(self);
    }
}
