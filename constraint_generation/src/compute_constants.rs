use crate::environment_utils::environment::ExecutionEnvironment as EE;
use crate::environment_utils::slice_types::{TagInfo, AExpressionSlice};
use circom_algebra::algebra::ArithmeticExpression;
use compiler::hir::very_concrete_program::{Argument, TemplateInstance};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use program_structure::ast::{Expression, Meta, Statement};
use program_structure::error_definition::ReportCollection;
use program_structure::program_archive::ProgramArchive;
use std::collections::{HashMap, BTreeMap, HashSet};
use crate::FlagsExecution;

type CCResult = Result<(), ReportCollection>;

struct Context<'a> {
    inside_template: bool,
    environment: &'a EE,
    program_archive: &'a ProgramArchive,
}

pub fn manage_functions(program_archive: &mut ProgramArchive, flags: FlagsExecution, prime: &String) -> CCResult {
    let mut reports = vec![];
    let mut processed = HashMap::new();

    for (name, data) in program_archive.get_functions() {
        let mut constant_variables = BTreeMap::new();
        let mut used_names = HashSet::new();
        let mut code = data.get_body().clone();
        let environment = EE::new();
        let context =
            Context { program_archive, inside_template: false, environment: &environment };
        treat_statement(&mut code, &context, &mut reports, &mut constant_variables, &mut used_names,  flags, prime);
        processed.insert(name.clone(), (code, constant_variables));
    }
    for (k, (v, ctes)) in processed {
        let function_data = program_archive.get_mut_function_data(&k);
        function_data.replace_body(v);
        // insert the info about the cte values of the functions
        function_data.set_constant_variables(ctes);
        
    }
    
    if reports.is_empty() {
        Result::Ok(())
    } else {
        Result::Err(reports)
    }
}

pub fn compute_vct(
    instances: &mut Vec<TemplateInstance>,
    program_archive: &ProgramArchive,
    flags: FlagsExecution,
    prime: &String
) -> CCResult {
    let mut reports = vec![];

    for instance in instances {
        let mut constant_variables = BTreeMap::new();
        let mut used_names = HashSet::new();

        let environment = transform_header_into_environment(&instance.header);
        let context = Context { program_archive, inside_template: true, environment: &environment };
        treat_statement(&mut instance.code, &context, &mut reports, &mut constant_variables, &mut used_names, flags, prime);
        instance.set_constant_variables(constant_variables);

    }
    if reports.is_empty() {
        Result::Ok(())
    } else {
        Result::Err(reports)
    }
}

fn transform_header_into_environment(header: &[Argument]) -> EE {
    let mut execution_environment = EE::new();
    for arg in header {
        let name = arg.name.clone();
        let slice = argument_into_slice(arg);
        execution_environment.add_variable(&name, (TagInfo::new(), slice));
    }
    execution_environment
}

fn argument_into_slice(argument: &Argument) -> AExpressionSlice {
    use ArithmeticExpression::Number;
    let arithmetic_expressions: Vec<ArithmeticExpression<String>> =
        argument.values.iter().map(|v| Number { value: v.clone() }).collect();
    let dimensions = argument.lengths.clone();
    AExpressionSlice::new_array(dimensions, arithmetic_expressions)
}

fn treat_statement(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection, constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    if stmt.is_initialization_block() {
        treat_init_block(stmt, context, reports, constants, used_names, flags, prime)
    } else if stmt.is_block() {
        treat_block(stmt, context, reports, constants, used_names, flags, prime)
    } else if stmt.is_if_then_else() {
        treat_conditional(stmt, context, reports, constants, used_names, flags, prime)
    } else if stmt.is_while() {
        treat_while(stmt, context, reports, constants, used_names, flags, prime)
    } else if stmt.is_declaration(){
        treat_declaration(stmt, context, reports, constants, used_names, flags, prime)
    } else if stmt.is_substitution(){
        treat_substitution(stmt, context, reports, constants, flags, prime)
    } else{

    }
}

fn treat_init_block(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection,  constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    use Statement::InitializationBlock;
    if let InitializationBlock { initializations, .. } = stmt {
        for init in initializations {
            if init.is_declaration() {
                treat_declaration(init, context, reports, constants, used_names, flags, prime)
            }
            if init.is_substitution(){
                treat_substitution(init, context, reports, constants, flags, prime)
            }
        }
    } else {
        unreachable!()
    }
}

fn treat_block(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection,  constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    use Statement::Block;
    if let Block { stmts, .. } = stmt {
        for s in stmts {
            treat_statement(s, context, reports, constants, used_names, flags, prime);
        }
    } else {
        unreachable!()
    }
}

fn treat_while(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection, constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    use Statement::While;
    if let While { stmt, .. } = stmt {
        treat_statement(stmt, context, reports, constants, used_names,  flags, prime);
    } else {
        unreachable!()
    }
}

fn treat_conditional(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection,  constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    use Statement::IfThenElse;
    if let IfThenElse { if_case, else_case, .. } = stmt {
        treat_statement(if_case, context, reports, constants, used_names, flags, prime);
        if let Option::Some(s) = else_case {
            treat_statement(s, context, reports, constants, used_names, flags, prime);
        }
    } else {
        unreachable!()
    }
}

fn treat_declaration(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection,  constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, used_names: &mut HashSet<String>, flags: FlagsExecution, prime: &String) {
    use Statement::Declaration;
    use program_structure::ast::VariableType::AnonymousComponent;
    use program_structure::ast::VariableType::Var;
    if let Declaration { meta, dimensions, xtype, name, .. } = stmt {
        let mut concrete_dimensions = vec![];
        match  xtype {
            AnonymousComponent => {
                meta.get_mut_memory_knowledge().set_concrete_dimensions(vec![]);
            },
            
            _ => {
                for d in dimensions.iter_mut() {
                    let execution_response = treat_dimension(d, context, reports, flags, prime);
                    if let Option::Some(v) = execution_response {
                        concrete_dimensions.push(v);
                    } else {
                        report_invalid_dimension(meta, reports);
                    }
                }
            
                // in case it is a var anotate as a potential constant
                // TODO: have a set of used names for duplicates
                match xtype{
                    Var =>{
                        if used_names.contains(name){
                            constants.remove(name);
                        } else{
                            used_names.insert(name.clone());
                            let size = concrete_dimensions.iter().fold(1, |length, val| length * val);
                            
                            let initial_vec = vec![BigInt::from(0); size];
                            constants.insert(name.clone(), (concrete_dimensions.clone(), initial_vec));
                        }
                    },
                    _ =>{

                    }
                }

                meta.get_mut_memory_knowledge().set_concrete_dimensions(concrete_dimensions);

            }
        }
    } else {
        unreachable!()
    }
}

fn treat_substitution(stmt: &mut Statement, context: &Context, reports: &mut ReportCollection,  constants: &mut BTreeMap<String, (Vec<usize>, Vec<BigInt>)>, flags: FlagsExecution, prime: &String) {
    use Statement::Substitution;

    if let Substitution{rhe, var, is_initialization, access, ..} = stmt{
        treat_expression(rhe, context, reports, flags, prime);
        
        if constants.contains_key(var){
            if !*is_initialization || !access.is_empty(){
                constants.remove(var);
            } else{
                let (is_cte, value) = is_constant_expression(rhe);
                if !is_cte{
                    constants.remove(var);
                } else{
                    let (_, prev_value) = constants.get_mut(var).unwrap();
                    let mut value = value.unwrap();
                    let initial_size = value.len();
                    // Case not complete assignments, only store the saved values
                    for _ in initial_size..prev_value.len(){
                        value.push(BigInt::from(0));
                    }              

                    *prev_value =  value;
                }
            }
        }
    } else{
        unreachable!()
    }

}

fn treat_expression(
    expr: &mut Expression, context: &Context, reports: &mut ReportCollection, flags: FlagsExecution, prime: &String
){
    use Expression::{Number, UniformArray};
    if let UniformArray {meta, value, dimension} = expr{
        let execution_response = treat_dimension(&dimension, context, reports, flags, prime);
        if let Option::Some(v) = execution_response {
            **dimension = Number(meta.clone(), BigInt::from(v));
        } else {
            report_invalid_dimension(meta, reports);
        }
        treat_expression(value, context, reports, flags, prime)
    } else{
    }
}

fn treat_dimension(
    dim: &Expression,
    context: &Context,
    reports: &mut ReportCollection,
    flags: FlagsExecution, 
    prime: &String,
) -> Option<usize> {
    use crate::execute::execute_constant_expression;
    if context.inside_template && !dim.is_number() {
        Option::None
    } else if let Expression::Number(_, v) = dim {
        transform_big_int_to_usize(v)
    } else {
        let program = context.program_archive;
        let env = context.environment;
        let execution_result = execute_constant_expression(dim, program, env.clone(), flags, prime);
        match execution_result {
            Result::Err(mut r) => {
                reports.append(&mut r);
                Option::None
            }
            Result::Ok(v) => transform_big_int_to_usize(&v),
        }
    }
}

fn is_constant_expression(e: &Expression)-> (bool, Option<Vec<BigInt>>){
    use program_structure::ast::Expression::*;
    match e{
        Number(_, v) => (true, Some(vec![v.clone()])),
        ArrayInLine{values, ..}=>{
            let mut result = Vec::new();
            for v in values{
                let (res, val) = is_constant_expression(v);
                if !res {
                    return (false, None);
                } else{
                    result.append(&mut val.unwrap());
                }
            }

            (true, Some(result))
        }
        UniformArray {value, dimension, .. }=>{
            let (res, val) = is_constant_expression(value);
            let mut result = Vec::new();
            if !res{
                (false, None)
            } else{
                let (res_dim, val_dim) =  is_constant_expression(dimension);
                let val = val.unwrap();
                if res_dim{
                    let val_dim = val_dim.unwrap();
                    assert!(val_dim.len() == 1);
                    for _ in 0..val_dim[0].to_usize().unwrap(){
                        result.push(res);
                    }
                    (true, Some(val))
                } else{
                    (false, None)
                }
            }

        }
        _ => (false, None)
    }
}

fn transform_big_int_to_usize(v: &BigInt) -> Option<usize> {
    v.to_usize()
}

fn report_invalid_dimension(meta: &Meta, reports: &mut ReportCollection) {
    use program_structure::error_code::ReportCode;
    use program_structure::error_definition::Report;
    let error_code = ReportCode::InvalidArraySize(0);
    let msg = "Invalid array size".to_string();
    let mut report = Report::error(msg, error_code);
    let message = "This expression can not be used as an array size".to_string();
    report.add_primary(meta.file_location(), meta.get_file_id(), message);
    reports.push(report);
}
