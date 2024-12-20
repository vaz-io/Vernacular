use std::io::{self, Write};
use crate::tokenizer::Tokenizer;
use crate::parser::Parser;
use crate::generator::{BytecodeGenerator, OpCode, Value};
use std::collections::HashMap;
use crate::analyzer::{Analyzer, Type};

pub struct Runtime {
    tokenizer: Tokenizer,
    variables: HashMap<String, Value>,
    variable_types: HashMap<String, Type>,
    stack: Vec<Value>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            tokenizer: Tokenizer::new(""),
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn run_repl(&mut self) -> Result<(), String> {
        println!("Vernacular Runtime v0.1.0");
        println!("'.exit' is quit, '.load' is load, or enter code directly.");

        let mut input = String::new();
        let mut is_continuation = false;

        loop {
            if is_continuation {
                print!("... ");
            } else {
                print!("> ");
            }
            io::stdout().flush().unwrap();

            let mut line = String::new();
            io::stdin().read_line(&mut line).expect("Failed to read line");
            let line = line.trim_end();

            match line {
                ".exit" if !is_continuation => {
                    println!("Goodbye!");
                    break;
                }
                ".load" if !is_continuation => {
                    println!("Enter file path:");
                    let mut file_path = String::new();
                    io::stdin().read_line(&mut file_path).expect("Failed to read line");
                    let file_path = file_path.trim();
                    
                    self.run_file(file_path)?;
                    input.clear();
                    is_continuation = false;
                }
                _ => {
                    input.push_str(line);
                    input.push('\n');  // Add newline to maintain line structure
                    
                    if line.trim_end().ends_with('\\') {
                        is_continuation = true;
                    } else {
                        if !input.trim().is_empty() {
                            self.process_input(&input)?;
                        }
                        input.clear();
                        is_continuation = false;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn run_file(&mut self, file_path: &str) -> Result<(), String> {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                println!("Running file: {}", file_path);
                self.process_input(&content)
            }
            Err(e) => Err(format!("Error reading file '{}': {}", file_path, e)),
        }
    }

    fn process_input(&mut self, input: &str) -> Result<(), String> {
        // First, preprocess the input to handle line continuations
        let processed_input = self.preprocess_input(input)?;
        
        self.tokenizer = Tokenizer::new(&processed_input);
        let tokens = self.tokenizer.tokenize()?;
        
        // Create and run parser
        let mut parser = Parser::new(tokens.clone());
        let ast = parser.parse()?;
        
        // Run type checker with existing variables
        let mut analyzer = Analyzer::new();
        
        // Only copy variables that have explicit types
        for (name, _value) in &self.variables {
            let var_type = if let Some(declared_type) = self.variable_types.get(name) {
                match declared_type.as_str() {
                    "Whole" => Type::Whole,
                    "Decimal" => Type::Decimal,
                    "Text" => Type::Text,
                    "Truth" => Type::Truth,
                    "Nothing" => Type::Nothing,
                    _ => Type::Any,
                }
            } else {
                Type::Any
            };
            analyzer.variables.insert(name.clone(), var_type);
        }
        
        analyzer.analyze(&ast)?;
        
        // Generate and run bytecode
        let mut generator = BytecodeGenerator::new();
        let bytecode = generator.generate(ast.clone())?;
        
        // Debug output
        println!("Tokens:");
        for token in tokens {
            println!("  {}", token);
        }
        
        println!("\nAST:");
        for node in &ast {
            println!("  {:?}", node);
        }
        
        println!("\nBytecode:");
        for op in &bytecode {
            println!("  {:?}", op);
        }

        self.execute_bytecode(bytecode)
    }

    fn preprocess_input(&self, input: &str) -> Result<String, String> {
        let mut processed = String::new();
        let mut lines = input.lines().peekable();
        
        while let Some(line) = lines.next() {
            let trimmed = line.trim_end();
            if trimmed.ends_with('\\') {
                // Remove the \ and add a space
                processed.push_str(&trimmed[..trimmed.len()-1]);
                processed.push(' ');
            } else {
                // Add the line as-is
                processed.push_str(trimmed);
                // Only add newline if there's more content
                if lines.peek().is_some() {
                    processed.push('\n');
                }
            }
        }
        
        Ok(processed)
    }

    fn execute_bytecode(&mut self, bytecode: Vec<OpCode>) -> Result<(), String> {
        let mut stack: Vec<Value> = Vec::new();
        let mut ip = 0;

        while ip < bytecode.len() {
            match &bytecode[ip] {
                OpCode::StoreVar(name) => {
                    let value = stack.pop().ok_or("Stack underflow")?;
                    
                    if let Some(declared_type) = self.variable_types.get(name) {
                        // Skip type checking if we're storing null during declaration
                        if !matches!(value, Value::Null) {
                            let value_type = match &value {
                                Value::Number(n) => {
                                    if n.fract() == 0.0 { "Whole" } else { "Decimal" }
                                },
                                Value::String(_) => "Text",
                                Value::Boolean(_) => "Truth",
                                Value::Null => "Nothing",
                                Value::Object(ref class_name) => class_name,
                            };
                            
                            if declared_type != value_type {
                                return Err(format!("Type mismatch: cannot assign {} to variable of type {}", 
                                              value_type, declared_type));
                            }
                        }
                    }
                    
                    self.variables.insert(name.clone(), value);
                    Ok(())
                },
                OpCode::LoadVar(name) => {
                    // Only try to load if the variable exists
                    if let Some(value) = self.variables.get(name) {
                        stack.push(value.clone());
                        Ok(())
                    } else {
                        Err(format!("Undefined variable: {}", name))
                    }
                },
                OpCode::Push(value) => {
                    stack.push(value.clone());
                    Ok(())
                },
                OpCode::Pop => {
                    stack.pop();
                    Ok(())
                },
                OpCode::Duplicate => {
                    if let Some(value) = stack.last() {
                        stack.push(value.clone());
                    }
                    Ok(())
                },
                OpCode::Add => {
                    let b = stack.pop().ok_or("Stack underflow")?;
                    let a = stack.pop().ok_or("Stack underflow")?;
                    stack.push(self.binary_op(a, b, |x, y| x + y)?);
                    Ok(())
                },
                OpCode::Subtract => {
                    let b = stack.pop().ok_or("Stack underflow")?;
                    let a = stack.pop().ok_or("Stack underflow")?;
                    stack.push(self.binary_op(a, b, |x, y| x - y)?);
                    Ok(())
                },
                OpCode::Multiply => {
                    let b = stack.pop().ok_or("Stack underflow")?;
                    let a = stack.pop().ok_or("Stack underflow")?;
                    stack.push(self.binary_op(a, b, |x, y| x * y)?);
                    Ok(())
                },
                OpCode::Divide => {
                    let b = stack.pop().ok_or("Stack underflow")?;
                    let a = stack.pop().ok_or("Stack underflow")?;
                    stack.push(self.binary_op(a, b, |x, y| x / y)?);
                    Ok(())
                },
                OpCode::Jump(target) => {
                    ip = *target;
                    Ok(())
                },
                OpCode::JumpIfFalse(target) => {
                    if let Some(Value::Boolean(false)) = stack.last() {
                        ip = *target;
                        Ok(())
                    } else {
                        Ok(())
                    }
                },
                OpCode::ConvertToString => {
                    let value = stack.pop().ok_or("Stack underflow")?;
                    stack.push(Value::String(value.to_string()));
                    Ok(())
                },
                OpCode::Call(name, arg_count) => {
                    let mut args = Vec::new();
                    // Pop arguments in reverse order
                    for _ in 0..*arg_count {
                        if let Some(arg) = stack.pop() {
                            args.insert(0, arg);
                        }
                    }

                    match name.as_str() {
                        "show" => {
                            // Built-in show function
                            if let Some(value) = args.get(0) {
                                println!("{}", value);
                            }
                            stack.push(Value::Null); // show returns null
                        },
                        _ => {
                            return Err(format!("Unknown function: {}", name));
                        }
                    }
                    Ok(())
                },
                OpCode::Return => {
                    // TODO: Implement return
                    break;
                },
                OpCode::NewObject(_class_name) => {
                    // TODO: Implement object creation
                    return Err("Object creation not implemented yet".to_string());
                },
                OpCode::GetProperty(_name) => {
                    // TODO: Implement property access
                    return Err("Property access not implemented yet".to_string());
                },
                OpCode::SetProperty(_name) => {
                    // TODO: Implement property setting
                    return Err("Property setting not implemented yet".to_string());
                },
                OpCode::CheckType(type_name) => {
                    if let Some(var_name) = self.get_next_var_name(&bytecode[ip+1..]) {
                        self.variable_types.insert(var_name.clone(), type_name.clone());
                    }
                    Ok(())
                },
                OpCode::Cast(type_name) => {
                    if let Some(value) = stack.pop() {
                        let new_value = match (value.clone(), type_name.as_str()) {
                            (Value::Number(n), "Whole") => {
                                Value::Number(n.floor())
                            },
                            (Value::Number(n), "Decimal") => {
                                Value::Number(n)
                            },
                            (Value::String(s), "Text") => {
                                Value::String(s)
                            },
                            (Value::Boolean(b), "Truth") => {
                                Value::Boolean(b)
                            },
                            _ => return Err(format!("Cannot cast {:?} to {}", value, type_name)),
                        };
                        stack.push(new_value);
                    }
                    Ok(())
                },
                OpCode::Concat => {
                    let b = stack.pop().ok_or("Stack underflow")?;
                    let a = stack.pop().ok_or("Stack underflow")?;
                    stack.push(self.concat_values(a, b)?);
                    Ok(())
                },
                OpCode::Interpolate(part_count) => {
                    let mut result = String::new();
                    for _ in 0..*part_count {
                        if let Some(value) = stack.pop() {
                            result = value.to_string() + &result;
                        }
                    }
                    stack.push(Value::String(result));
                    Ok(())
                },
                OpCode::CheckAssignmentType => {
                    let _var_value = stack.pop().ok_or("Stack underflow")?;
                    let new_value = stack.last().ok_or("Stack underflow")?;
                    
                    if let Some(var_name) = self.get_next_var_name(&bytecode[ip+1..]) {
                        // Only check type if the variable has an explicit type declaration
                        if let Some(declared_type) = self.variable_types.get(&var_name) {
                            let new_type = match new_value {
                                Value::Number(n) => {
                                    if n.fract() == 0.0 { Type::Whole } else { Type::Decimal }
                                },
                                Value::String(_) => Type::Text,
                                Value::Boolean(_) => Type::Truth,
                                Value::Null => Type::Nothing,
                                Value::Object(ref class_name) => Type::Object,
                                Value::Promise(ref class_name) => Type::Promise,
                                Value::List(ref class_name) => Type::List,
                                Value::Mapping(ref class_name) => Type::Mapping,
                            };

                            if declared_type != new_type {
                                return Err(format!("Type mismatch: cannot assign {} to variable of type {}", 
                                              new_type, declared_type));
                            }
                        }
                        // If variable doesn't have a declared type, allow any assignment
                    }
                    Ok(())
                },
                OpCode::Show => {
                    if let Some(value) = stack.pop() {
                        println!("{}", value);
                    } else {
                        return Err("Stack underflow".to_string());
                    }
                    Ok(())
                },
            }?;
            ip += 1;
        }
        Ok(())
    }

    fn get_next_var_name(&self, upcoming_ops: &[OpCode]) -> Option<String> {
        for op in upcoming_ops {
            if let OpCode::StoreVar(name) = op {
                return Some(name.clone());
            }
        }
        None
    }

    // Helper methods for the Runtime impl
    fn binary_op<F>(&self, a: Value, b: Value, op: F) -> Result<Value, String>
    where
        F: Fn(f64, f64) -> f64,
    {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(Value::Number(op(x, y))),
            _ => Err("Invalid operands for arithmetic operation".to_string()),
        }
    }

    fn concat_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (a, b) {
            (Value::String(s1), Value::String(s2)) => Ok(Value::String(s1 + &s2)),
            _ => Err("Can only concatenate strings".to_string()),
        }
    }

    fn execute(&mut self, instructions: &[OpCode]) -> Result<(), String> {
        for instruction in instructions {
            match instruction {
                OpCode::Show => {
                    if let Some(value) = self.stack.pop() {
                        println!("{}", value);
                    }
                },
                OpCode::Push(value) => {
                    self.stack.push(value.clone());
                },
                OpCode::LoadVar(name) => {
                    if let Some(value) = self.variables.get(name) {
                        self.stack.push(value.clone());
                    } else {
                        return Err(format!("Undefined variable: {}", name));
                    }
                },
                OpCode::StoreVar(name) => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    
                    // Check type if variable has a declared type
                    if let Some(declared_type) = self.variable_types.get(name) {
                        let value_type = match &value {
                            Value::Number(n) => {
                                if n.fract() == 0.0 { Type::Whole } else { Type::Decimal }
                            },
                            Value::String(_) => Type::Text,
                            Value::Boolean(_) => Type::Truth,
                            Value::Null => Type::Nothing,
                            Value::Object(_) => Type::Object,
                            Value::Promise(_) => Type::Promise(Box::new(Type::Any)),
                            Value::List(_) => Type::List(Box::new(Type::Any)),
                            Value::Mapping(_) => Type::Map { key: Box::new(Type::Text), value: Box::new(Type::Any) },
                        };
                        
                        if declared_type != &value_type {
                            return Err(format!("Type mismatch: cannot assign {:?} to variable of type {:?}", 
                                value_type, declared_type));
                        }
                    }
                    
                    self.variables.insert(name.clone(), value);
                },
                OpCode::Add | OpCode::Subtract | OpCode::Multiply | OpCode::Divide => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = match instruction {
                        OpCode::Add => self.binary_op(a, b, |x, y| x + y)?,
                        OpCode::Subtract => self.binary_op(a, b, |x, y| x - y)?,
                        OpCode::Multiply => self.binary_op(a, b, |x, y| x * y)?,
                        OpCode::Divide => self.binary_op(a, b, |x, y| x / y)?,
                        _ => unreachable!(),
                    };
                    self.stack.push(result);
                },
                OpCode::Pop => {
                    self.stack.pop();
                },
                OpCode::Duplicate => {
                    if let Some(value) = self.stack.last() {
                        self.stack.push(value.clone());
                    }
                },
                _ => return Err(format!("Unhandled opcode: {:?}", instruction)),
            }
        }
        Ok(())
    }
}


fn main() -> Result<(), String> {
    let mut runtime = Runtime::new();
    runtime.run_repl()
}
