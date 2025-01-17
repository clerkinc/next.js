use serde::Deserialize;

use turbo_binding::swc::core::{
    common::Mark,
    ecma::ast::*,
    ecma::atoms::{js_word, JsWord},
    ecma::transforms::optimization::simplify::dce::{dce, Config as DCEConfig},
    ecma::visit::{Fold, FoldWith},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub ignore: Vec<JsWord>,
}

pub fn shake_exports(config: Config) -> impl Fold {
    ExportShaker {
        ignore: config.ignore,
        ..Default::default()
    }
}

#[derive(Debug, Default)]
struct ExportShaker {
    ignore: Vec<JsWord>,
    remove_export: bool,
}

impl Fold for ExportShaker {
    fn fold_module(&mut self, module: Module) -> Module {
        let module = module.fold_children_with(self);
        module.fold_with(&mut dce(DCEConfig::default(), Mark::new()))
    }

    fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
        let mut new_items = vec![];
        for item in items {
            let item = item.fold_children_with(self);
            if !self.remove_export {
                new_items.push(item)
            }
            self.remove_export = false;
        }
        new_items
    }

    fn fold_export_decl(&mut self, mut decl: ExportDecl) -> ExportDecl {
        match &mut decl.decl {
            Decl::Fn(fn_decl) => {
                if !self.ignore.contains(&fn_decl.ident.sym) {
                    self.remove_export = true;
                }
            }
            Decl::Class(class_decl) => {
                if !self.ignore.contains(&class_decl.ident.sym) {
                    self.remove_export = true;
                }
            }
            Decl::Var(var_decl) => {
                var_decl.decls = var_decl
                    .decls
                    .iter()
                    .filter_map(|var_decl| {
                        if let Pat::Ident(BindingIdent { id, .. }) = &var_decl.name {
                            if self.ignore.contains(&id.sym) {
                                return Some(var_decl.to_owned());
                            }
                        }
                        None
                    })
                    .collect();
                if var_decl.decls.is_empty() {
                    self.remove_export = true;
                }
            }
            _ => {}
        }
        decl
    }

    fn fold_named_export(&mut self, mut export: NamedExport) -> NamedExport {
        export.specifiers = export
            .specifiers
            .into_iter()
            .filter_map(|spec| {
                if let ExportSpecifier::Named(named_spec) = spec {
                    if let Some(ident) = &named_spec.exported {
                        if let ModuleExportName::Ident(ident) = ident {
                            if self.ignore.contains(&ident.sym) {
                                return Some(ExportSpecifier::Named(named_spec));
                            }
                        }
                    } else if let ModuleExportName::Ident(ident) = &named_spec.orig {
                        if self.ignore.contains(&ident.sym) {
                            return Some(ExportSpecifier::Named(named_spec));
                        }
                    }
                }
                None
            })
            .collect();
        if export.specifiers.is_empty() {
            self.remove_export = true
        }
        export
    }

    fn fold_export_default_decl(&mut self, decl: ExportDefaultDecl) -> ExportDefaultDecl {
        if !self.ignore.contains(&js_word!("default")) {
            self.remove_export = true
        }
        decl
    }

    fn fold_export_default_expr(&mut self, expr: ExportDefaultExpr) -> ExportDefaultExpr {
        if !self.ignore.contains(&js_word!("default")) {
            self.remove_export = true
        }
        expr
    }
}
