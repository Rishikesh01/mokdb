use crate::{
    engine::semantic_analysis::{Scope, SemanticAnalyzer},
    parser::ast::Select,
    storage::catalog_manager,
};

mod engine;
mod errors;
mod parser;
mod storage;

fn main() {
    println!("Hello, world!");
    let semantic_analyzer = SemanticAnalyzer::new(catalog_manager::CatalogManagerImpl::new(vec![]));
    semantic_analyzer.analyze(
        Scope::default(),
        parser::ast::Statement::SelectStmt(Select {
            with: None,
            distinct: None,
            projection: todo!(),
            from: todo!(),
            where_clause: todo!(),
            group_by: todo!(),
            order_by: todo!(),
            limit: todo!(),
            offset: todo!(),
        }),
    );
}
