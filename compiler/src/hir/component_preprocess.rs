use crate::hir::very_concrete_program::VCP;
use program_structure::ast::*;
use std::collections::BTreeMap;
use num_bigint_dig::BigInt;

pub fn rm_component_ci(vcp: &mut VCP) {
    for template in &mut vcp.templates {
        rm_statement(&mut template.code, &template.constant_variables);
    }
    for function in &mut vcp.functions {
        rm_statement(&mut function.body, &function.constant_variables);
    }
}

fn rm_statement(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) {
    if stmt.is_while() {
        rm_while(stmt, constants);
    } else if stmt.is_if_then_else() {
        rm_if_then_else(stmt, constants);
    } else if stmt.is_block() {
        rm_block(stmt, constants);
    } else if stmt.is_initialization_block() {
        rm_init(stmt, constants);
    } else if stmt.is_substitution(){ 
        rm_substitution(stmt, constants);
    } else if stmt.is_underscore_substitution(){ 
        rm_underscore_substitution(stmt, constants);
    }
}

fn rm_underscore_substitution(stmt: &mut Statement, _constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>){
    use Statement::{Block, UnderscoreSubstitution};
    if let UnderscoreSubstitution { meta, .. } = stmt{
        *stmt = Block{ meta: meta.clone(), stmts: Vec::new() };
    }
}

fn rm_block(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) {
    use Statement::Block;
    if let Block { stmts, .. } = stmt {
        let filter = std::mem::take(stmts);
        for mut s in filter {
            rm_statement(&mut s, constants);
            if !should_be_removed(&s, constants) {
                stmts.push(s);
            }
        }
    } else {
        unreachable!()
    }
}

fn rm_if_then_else(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) {
    use Statement::IfThenElse;
    if let IfThenElse { if_case, else_case, .. } = stmt {
        rm_statement(if_case, constants);
        if let Option::Some(s) = else_case {
            rm_statement(s, constants);
        }
    } else {
        unreachable!()
    }
}

fn rm_while(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) {
    use Statement::While;
    if let While { stmt, .. } = stmt {
        rm_statement(stmt, constants);
    } else {
        unreachable!()
    }
}

fn rm_init(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) {
    use Statement::InitializationBlock;
    use VariableType::*;
    if let InitializationBlock { initializations, xtype, .. } = stmt {

        if let Signal(..) = xtype  {
            let work = std::mem::take(initializations);
            for mut i in work {
                if i.is_substitution() {
                    initializations.push(i);
                } else if i.is_block(){
                    rm_block(&mut i, constants);
                    initializations.push(i);
                }
            }
        } else if let Bus(..) = xtype{
            let work = std::mem::take(initializations);
            for mut i in work {
                if i.is_substitution() {
                    if !should_be_removed(&i, constants) {
                        initializations.push(i);
                    }
                } else if i.is_block(){
                    rm_block(&mut i, constants);
                    initializations.push(i);
                }
            }
        }else {
            let filter = std::mem::take(initializations);
            for mut s in filter {
                rm_statement(&mut s, constants);
                if !should_be_removed(&s, constants) {
                    initializations.push(s);
                }
            }
        }
    } else {
        unreachable!()
    }
}

fn rm_substitution(stmt: &mut Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>){
    use Statement::{Block, Substitution};
    if should_be_removed(stmt, constants){
        if let Substitution { meta, .. } = stmt{
            *stmt = Block{ meta: meta.clone(), stmts: Vec::new() };
        }
    }
}

fn should_be_removed(stmt: &Statement, constants: &BTreeMap<String, (Vec<usize>, Vec<BigInt>)>) -> bool {
    use Statement::{InitializationBlock, Substitution};
    use VariableType::*;
    if let InitializationBlock { xtype, .. } = stmt {
        Component == *xtype || AnonymousComponent == *xtype
    } else if let Substitution { meta, rhe, var, is_initialization, .. } = stmt {
            meta.get_type_knowledge().is_component() 
            || meta.get_type_knowledge().is_tag()
            || rhe.is_bus_call() || rhe.is_bus_call_array()
            || (*is_initialization && constants.contains_key(var))
    } else {
        false
    }
}
