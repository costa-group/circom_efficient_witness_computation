use super::ir_interface::*;
use crate::translating_traits::*;
use code_producers::c_elements::*;
use code_producers::wasm_elements::*;

#[derive(Clone)]
pub struct LoadConstantBucket {
    pub line: usize,
    pub message_id: usize,
    pub variable_name: String,
    pub location: InstructionPointer,
}

impl IntoInstruction for LoadConstantBucket {
    fn into_instruction(self) -> Instruction {
        Instruction::LoadConstant(self)
    }
}

impl Allocate for LoadConstantBucket {
    fn allocate(self) -> InstructionPointer {
        InstructionPointer::new(self.into_instruction())
    }
}

impl ObtainMeta for LoadConstantBucket {
    fn get_line(&self) -> usize {
        self.line
    }
    fn get_message_id(&self) -> usize {
        self.message_id
    }
}

impl ToString for LoadConstantBucket {
    fn to_string(&self) -> String {
        let line = self.line.to_string();
        let template_id = self.message_id.to_string();
        let address = self.variable_name.to_string();
        let src = self.location.to_string();
        format!(
            "LOADCONSTANT(line:{},template_id:{},address_type:{},src:{})",
            line, template_id, address, src
        )
    }
}
impl WriteWasm for LoadConstantBucket {
    fn produce_wasm(&self, producer: &WASMProducer) -> Vec<String> {
        
        vec![]
    }
}

impl WriteC for LoadConstantBucket {
    fn produce_c(&self, producer: &CProducer, parallel: Option<bool>) -> (Vec<String>, String) {
        let (prologue, value_index) = self.location.produce_c(producer, parallel);
        
        let accessed_value = format!("{}[{}]",
            self.variable_name,
            value_index
        );
        
	    //prologue.push(format!("// end of load line {} with access {}",self.line.to_string(),access));
        (prologue, accessed_value)
    }
}
