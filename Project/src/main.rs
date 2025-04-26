use std::env;
use std::fs::File;
use std::io::{self, Read};

// Define i64 as the default integer type to match C's `long long`
type Int = i64;

// Tokens and classes (operators in precedence order)
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Token {
    Num = 128, Fun, Sys, Glo, Loc, Id,
    Char, Else, Enum, If, Int, Return, Sizeof, While,
    Assign, Cond, Lor, Lan, Or, Xor, And, Eq, Ne, Lt, Gt, Le, Ge, Shl, Shr, Add, Sub, Mul, Div, Mod, Inc, Dec,
    OPEN, READ, CLOS, PRTF, MALC, FREE, MSET, MCMP, EXIT,
    Brak, Semicolon, CurlyOpen, CurlyClose, Comma,
}

// Opcodes for the virtual machine
#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(dead_code)]
enum Opcode {
    LEA, IMM, JMP, JSR, BZ, BNZ, ENT, ADJ, LEV, LI, LC, SI, SC, PSH,
    OR, XOR, AND, EQ, NE, LT, GT, LE, GE, SHL, SHR, ADD, SUB, MUL, DIV, MOD,
    OPEN, READ, CLOS, PRTF, MALC, FREE, MSET, MCMP, EXIT,
}

// Types
#[derive(Debug, PartialEq, Clone, Copy)]
enum Type {
    CHAR = 0, INT, PTR,
}

// Identifier struct for symbol table
#[derive(Debug, PartialEq, Clone)]
struct Ident {
    tk: Token,
    hash: Int,
    name: String,
    class: Token,
    ty: Type,
    val: Int,
    hclass: Token,
    hty: Type,
    hval: Int,
}

#[allow(dead_code)]
struct Compiler {
    src: String,
    p: usize,
    lp: usize,
    data: Vec<u8>,
    e: Vec<Int>,
    le: usize,
    sym: Vec<Ident>,
    tk: Option<Token>,
    ival: Int,
    ty: Type,
    loc: Int,
    line: Int,
    src_flag: bool,
    debug: bool,
}
#[allow(dead_code)]
impl Compiler {
    fn new(src: String, poolsz: usize) -> Self {
        Compiler {
            src,
            p: 0,
            lp: 0,
            data: vec![0; poolsz],
            e: vec![0; poolsz],
            le: 0,
            sym: Vec::new(),
            tk: None,
            ival: 0,
            ty: Type::INT,
            loc: 0,
            line: 1,
            src_flag: false,
            debug: false,
        }
    }

// PUT YOUR CODE HERE
//try 
    fn stmt(&mut self) {
        // wen parse a single statement in the source code, like an if, while, or assignment and it checks the current token to decide what kind of statement it is
        match self.tk {   // self.tk holds the current token we're looking at
            // if we get an if statement and lets the program choose between two paths based on the given condition
            Some(Token::If) => {
                // move to the token after if to process the condition
                // self.next() updates self.tk to the next token
                self.next();
                // make sure the next token is a parenthesis and if it's not panic give an error message showing the line number for debugging
                if self.tk != Some(Token::Brak) { panic!("{}: open paren expected", self.line); }
                // move past to parse the condition
                self.next();
                // parse the condition expression for assignment
                // generate bytecode for the expression and leaves the result on the stack
                self.expr(Token::Assign);
                // check for a closing parenthesis error if not there
                if self.tk != Some(Token::Brak) { panic!("{}: close paren expected", self.line); }
                // move
                self.next();
                // use a Branch-if-Zero (BZ) opcode to check if the condition result is F 0
                // if 0 skip to next part of the code
                self.e.push(Opcode::BZ as Int);
                // save the current length of the bytecode array (self.e) to patch the jump target later, leave a placeholder for where to jump if the condition fails
                let b = self.e.len();
                // push a temp 0 as the jump target and later fill in the actual address after parsing the if body
                self.e.push(0);
                // recursively parse the if which could be a single statement or a block of statements
                self.stmt();
                // check if there's an else to handle the other path
                if self.tk == Some(Token::Else) {
                    // if there's an else the update the BZ jump target to skip the else branch if the condition was true
                    // the jump target is set to the current bytecode length plus 3
                    self.e[b] = (self.e.len() + 3) as Int;
                    // a Jump (JMP) opcode to skip the else branch if you used the if to make sure the else code is only run if the condition was false
                    self.e.push(Opcode::JMP as Int);
                    // save the index for the JMP target to patch later
                    let b_else = self.e.len();
                    // push a temp 0 for the JMP target and set it after parsing the else
                    self.e.push(0);
                    // move
                    self.next();
                    // parse the else body which could be another statement or block
                    self.stmt();
                    // patch the JMP target to point to the instruction that comes after the else body to run the program after the if else
                    self.e[b_else] = (self.e.len() + 1) as Int;
                } else {
                    // if there's no else patch the BZ target to jump to the instruction after the if body, this skips the if body if the condition was false and continues with the next statement
                    self.e[b] = (self.e.len() + 1) as Int;
                }
            }
            // for a while loop to repeat code as long as a condition is true
            Some(Token::While) => {
                // move past the while token to process the condition
                self.next();
                // save the starting point of the loop (bytecode index + 1) for jumping back to the condition to return to
                let a = self.e.len() + 1;
                // check for an opening parenthesis error if missing
                if self.tk != Some(Token::Brak) { panic!("{}: open paren expected", self.line); }
                // move past to parse the condition
                self.next();
                // parse the loop condition to allow assignments and generate a bytecode for it
                self.expr(Token::Assign);
                // expect a closing parenthesis error if missing
                if self.tk != Some(Token::Brak) { panic!("{}: close paren expected", self.line); }
                // move
                self.next();
                // use a BZ opcode to exit the loop if the condition is false
                self.e.push(Opcode::BZ as Int);
                // save the index for the BZ jump target to patch later
                let b = self.e.len();
                // push a temp 0 for the BZ target
                self.e.push(0);
                // parse the loop body which could be a single statement or a block
                self.stmt();
                // emit a Jump opcode to return to the start of the loop
                self.e.push(Opcode::JMP as Int);
                // set the JMP target to the saved starting point to loop back to the condition
                self.e.push(a as Int);
                // patch the BZ target to jump to the instruction after the loop body if the condition is false
                self.e[b] = (self.e.len() + 1) as Int;
            }
            // for a return statement to exit a function and give back a value if needed
            Some(Token::Return) => {
                // move past the return token
                self.next();
                // check if there's an expression to return and if the next token isn't a semicolon then parse the expression
                if self.tk != Some(Token::Semicolon) { self.expr(Token::Assign); }
                // use a LEV opcode to exit the function and clean the stack and return control
                self.e.push(Opcode::LEV as Int);
                // expect a semicolon to end the statement error if none
                if self.tk != Some(Token::Semicolon) { panic!("{}: semicolon expected", self.line); }
                // move
                self.next();
            }
            // for a block of statements in curly braces
            Some(Token::CurlyOpen) => {
                // move past the opening {
                self.next();
                // keep parsing statements until you get to }
                while self.tk != Some(Token::CurlyClose) { self.stmt(); }
                // move past the }
                self.next();
            }
            // semicolon
            Some(Token::Semicolon) => {
                // move past it only
                self.next();
            }
            // treat the statement as an expression like an assignment or function call
            _ => {
                // parse the expression to allow assignments and generate bytecode
                self.expr(Token::Assign);
                // expect a semicolon to end the expression statement error if none
                if self.tk != Some(Token::Semicolon) { panic!("{}: semicolon expected", self.line); }
                // move
                self.next();
            }
        }
    }
    
    fn run(&mut self) -> i32 {
        // execute the compiled bytecode in self.e to simulate a virtual machine
        // initialize a program counter to point to the first instruction
        let mut pc = 0;
        // make a stack for function calls, local variables, and intermediate results with 256 KB size
        let mut sp = vec![0; 256 * 1024];
        // set the base pointer to the top of the stack to manage function frames
        let mut bp = sp.len();
        // initialize the accumulator to store temp results of operations
        let mut a: Int = 0;
        // track the number of executed instructions for debugging
        let mut cycle = 0;
    
        // loop until the program counter reaches the end of the bytecode array
        while pc < self.e.len() {
            // fetch the current instruction from the bytecode array
            let i = self.e[pc];
            // increment the program counter to point to the next instruction
            pc += 1;
            // increment the cycle counter to track execution progress
            cycle += 1;
    
            // if debug mode is on print the current instruction to trace execution
            if self.debug {
                // turn the opcode into a readable string for debugging like 0 to LEA
                let op_str = match i {
                    0 => "LEA", 1 => "IMM", 2 => "JMP", 3 => "JSR", 4 => "BZ", 5 => "BNZ",
                    6 => "ENT", 7 => "ADJ", 8 => "LEV", 9 => "LI", 10 => "LC", 11 => "SI",
                    12 => "SC", 13 => "PSH", 14 => "OR", 15 => "XOR", 16 => "AND", 17 => "EQ",
                    18 => "NE", 19 => "LT", 20 => "GT", 21 => "LE", 22 => "GE", 23 => "SHL",
                    24 => "SHR", 25 => "ADD", 26 => "SUB", 27 => "MUL", 28 => "DIV", 29 => "MOD",
                    30 => "OPEN", 31 => "READ", 32 => "CLOS", 33 => "PRTF", 34 => "MALC",
                    35 => "FREE", 36 => "MSET", 37 => "MCMP", 38 => "EXIT", _ => "???",
                };
                // print the cycle number and opcode name to track whats running
                print!("{}> {:4}", cycle, op_str);
                // if the opcode has an operand like a jump address print it else just newline
                if i <= 7 { println!(" {}", self.e[pc]); } else { println!(); }
            }
    
            // bring in libc functions for system stuff like file operations or memory management
            #[allow(unused_imports)]
            use libc::{open, read, close, malloc, free, memset, memcmp, c_void};
    
            // run the instruction based on its opcode value
            match i {
                //   LEA compute an address relative to the base pointer and store in accumulator to access local variables or parameters
                0 => a = (bp as Int + self.e[pc]) as Int,
                //   IMM load a literal value from the next instruction into accumulator like a constant number
                1 => a = self.e[pc],
                //   JMP jump to the instruction address in the next instruction to move to another part of the code
                2 => pc = self.e[pc] as usize,
                //   JSR jump to a subroutine and push the return address (pc + 1) to the stack to come back later
                3 => { bp -= 1; sp[bp] = (pc + 1) as Int; pc = self.e[pc] as usize; },
                //   BZ jump to the address if accumulator is 0 else keep going to skip blocks like in if statements
                4 => pc = if a == 0 { self.e[pc] as usize } else { pc + 1 },
                //   BNZ jump to the address if accumulator is not 0 else keep going for conditions that need to be true
                5 => pc = if a != 0 { self.e[pc] as usize } else { pc + 1 },
                //   ENT enter a function by saving the base pointer and allocating stack space to set up a new stack frame
                6 => { bp -= 1; sp[bp] = bp as Int; bp = (bp as Int - self.e[pc]) as usize; pc += 1; },
                //   ADJ adjust the stack pointer to deallocate temp stack space after a function call to clean up arguments
                7 => bp = (bp as Int + self.e[pc]) as usize,
                //   LEV leave a function by restoring the base pointer and jumping to the return address to pop the stack frame
                8 => { bp = sp[bp] as usize; pc = sp[bp + 1] as usize; bp += 2; },
                //   LI load an integer from the memory address in the accumulator to read a variable value
                9 => a = unsafe { *(a as *const Int) },
                //   LC load a character from the memory address in the accumulator and cast to integer
                10 => a = unsafe { *(a as *const u8) as Int },
                //   SI store the  accumulators integer to the memory address at stack top and pop the address
                11 => unsafe { *(sp[bp] as *mut Int) = a; bp += 1; },
                //   SC store the  accumulators value as a character to the memory address at stack top and pop
                12 => { unsafe { *(sp[bp] as *mut u8) = a as u8; } bp += 1; },
                //   PSH push the  accumulators value to the stack to save it   later use like in calculations
                13 => { bp -= 1; sp[bp] = a; },
                //   OR do a bitwise OR between stack top and accumulator and store in accumulator for logical ops
                14 => a = sp[bp] | a,
                //   XOR do a bitwise XOR between stack top and accumulator to toggle bits
                15 => a = sp[bp] ^ a,
                //   AND do a bitwise AND between stack top and accumulator for masking bits
                16 => a = sp[bp] & a,
                //   EQ set accumulator to 1 if stack top equals accumulator else 0 for equality checks
                17 => a = if sp[bp] == a { 1 } else { 0 },
                //   NE set accumulator to 1 if stack top not equal to accumulator else 0 for inequality
                18 => a = if sp[bp] != a { 1 } else { 0 },
                //   LT set accumulator to 1 if stack top less than accumulator else 0 for less-than checks
                19 => a = if sp[bp] < a { 1 } else { 0 },
                //   GT set accumulator to 1 if stack top greater than accumulator else 0 for greater-than
                20 => a = if sp[bp] > a { 1 } else { 0 },
                //   LE set accumulator to 1 if stack top less or equal to accumulator else 0
                21 => a = if sp[bp] <= a { 1 } else { 0 },
                //   GE set accumulator to 1 if stack top greater or equal to accumulator else 0
                22 => a = sp[bp] >= a { 1 } else { 0 },
                //   SHL shift stack top left by  accumulators value for bitwise ops like multiplying by 2
                23 => a = sp[bp] << a,
                //   SHR shift stack top right by  accumulators value for division or bit extraction
                24 => a = sp[bp] >> a,
                //   ADD add stack top and accumulator and store in accumulator for addition
                25 => a = sp[bp] + a,
                //   SUB subtract accumulator from stack top and store in accumulator for subtraction
                26 => a = sp[bp] - a,
                //   MUL multiply stack top and accumulator and store in accumulator
                27 => a = sp[bp] * a,
                //   DIV divide stack top by accumulator and store quotient in accumulator
                28 => a = sp[bp] / a,
                //   MOD compute remainder of stack top divided by accumulator
                29 => a = sp[bp] % a,
                //   OPEN call  systems open() to open a file using filename and flags from stack and return file descriptor
                30 => a = unsafe { open(sp[bp + 1] as *const i8, sp[bp] as i32) as Int },
                //   READ call  systems read() to read from a file using file descriptor, buffer, and size from stack
                31 => a = unsafe { read(sp[bp + 2] as i32, sp[bp + 1] as *mut c_void, sp[bp] as usize) as Int },
                //   CLOS call  systems close() to close a file using file descriptor from stack
                32 => a = unsafe { close(sp[bp] as i32) as Int },
                //   PRTF call printf to print formatted output using format string and up to six args from stack
                33 => {
                    let t = &sp[bp + self.e[pc + 1] as usize..];
                    a = unsafe { libc::printf(t[t.len() - 1] as *const i8, t[t.len() - 2], t[t.len() - 3], t[t.len() - 4], t[t.len() - 5], t[t.len() - 6]) as Int };
                },
                //   MALC allocate memory with malloc using size from stack and return address in accumulator
                34 => a = unsafe { libc::malloc(sp[bp] as usize) as Int },
                //   FREE free memory with free using address from stack
                35 => unsafe { libc::free(sp[bp] as *mut libc::c_void) },
                //   MSET set a memory block to a value with memset using address, value, and size from stack
                36 => unsafe { libc::memset(sp[bp + 2] as *mut libc::c_void, sp[bp + 1] as i32, sp[bp] as usize); },
                //   MCMP compare two memory blocks with memcmp using addresses and size from stack
                37 => a = unsafe { libc::memcmp(sp[bp + 2] as *const libc::c_void, sp[bp + 1] as *const libc::c_void, sp[bp] as usize) as Int },
                //   EXIT stop the program with exit code from stack top and print debug info
                38 => { println!("exit({}) cycle = {}", sp[bp], cycle); return sp[bp] as i32; },
                // if opcode not recognized print error with instruction and cycle count and return -1
                _ => { println!("unknown instruction = {}! cycle = {}", i, cycle); return -1; }
            }
            //   opcodes with an operand like jump addresses move past it to next instruction
            if i <= 7 { pc += 1; }
            //   binary ops like math or comparisons pop the stack top after the op to remove one operand
            if i >= 14 && i <= 29 { bp += 1; }
        }
        // if execution finishes without exit return 0 to show success
        0
    }
    
    fn main() -> io::Result<()> {
        // start the program to set up and run the compiler
        // grab command-line args but skip the program name like c4 in ./c4 file.c
        let mut args: Vec<String> = env::args().skip(1).collect();
        // set flag to print source code for debugging
        let mut src_flag = false;
        // set flag to show debug output during bytecode execution
        let mut debug = false;
    
        // check if -s flag is there to print source code
        if args.iter().any(|arg| arg == "-s") {
            src_flag = true;
            // remove -s from args to keep just the file name
            args.retain(|arg| arg != "-s");
        }
        // check if -d flag is there to show debug output
        if args.iter().any(|arg| arg == "-d") {
            debug = true;
            // remove -d from args
            args.retain(|arg| arg != "-d");
        }
        // if no file name is left after flags print usage and exit with error
        if args.is_empty() {
            println!("usage: c4 [-s] [-d] file ...");
            std::process::exit(-1);
        }
    
        // open the source file from the first arg like file.c
        let mut file = File::open(&args[0])?;
        // read the whole file into a string to process it
        let mut src = String::new();
        file.read_to_string(&mut src)?;
    
        // set memory pool size to 256 KB for compilers data like symbol table
        let poolsz = 256 * 1024;
        // make a new compiler with the source code and memory pool size to set up tokenizing and parsing
        let mut compiler = Compiler::new(src, poolsz);
    
        // set compiler flags for source printing and debug output
        compiler.src_flag = src_flag;
        compiler.debug = debug;
    
        // list the languages keywords and system calls as a string
        let keywords = "char else enum if int return sizeof while open read close printf malloc free memset memcmp exit void main";
        // split keywords into individual words to process
        let mut p = keywords.split_whitespace();
        // add keywords like char, if, while to the symbol table to recognize them
        for tk in [Token::Char, Token::Else, Token::Enum, Token::If, Token::Int, Token::Return, Token::Sizeof, Token::While] {
            let name = p.next().unwrap().to_string();
            compiler.sym.push(Ident { tk, hash: 0, name, class: tk, ty: Type::INT, val: 0, hclass: tk, hty: Type::INT, hval: 0 });
        }
        // add system calls like open, printf to the symbol table with opcode values starting from 30
        for (tk, val) in [Token::OPEN, Token::READ, Token::CLOS, Token::PRTF, Token::MALC, Token::FREE, Token::MSET, Token::MCMP, Token::EXIT].iter().zip(30..) {
            let name = p.next().unwrap().to_string();
            compiler.sym.push(Ident { tk: *tk, hash: 0, name, class: Token::Sys, ty: Type::INT, val: val as Int, hclass: Token::Sys, hty: Type::INT, hval: 0 });
        }
    
        // start tokenizing by getting the first token from source code
        compiler.next();
        // parse global declarations like variables or enums until no tokens left
        while compiler.tk.is_some() {
            // assume variables are integers unless told otherwise
            let mut bt = Type::INT;
            // check for type specifiers like int, char, or enum
            if compiler.tk == Some(Token::Int) {
                // if token is int move to next token
                compiler.next();
            } else if compiler.tk == Some(Token::Char) {
                // if token is char set base type to CHAR and move
                compiler.next();
                bt = Type::CHAR;
            } else if compiler.tk == Some(Token::Enum) {
                // for enum declaration to define named constants
                compiler.next();
                // if next token isnt a brace skip optional enum name like enum Color
                if compiler.tk != Some(Token::CurlyOpen) { compiler.next(); }
                if compiler.tk == Some(Token::CurlyOpen) {
                    // if brace is found parse the enum constants
                    compiler.next();
                    let mut i = 0;
                    // process each of the constants until closing brace
                    while compiler.tk != Some(Token::CurlyClose) {
                        // expect an identifier for a constant name and give a error if none
                        if compiler.tk != Some(Token::Id) { panic!("{}: bad enum identifier {:?}", compiler.line, compiler.tk); }
                        // grab the identifier name from source code
                        let name = compiler.src[compiler.p - 1..compiler.p].to_string();
                        compiler.next();
                        // if theres an = sign use the given value for the constant
                        if compiler.tk == Some(Token::Assign) {
                            compiler.next();
                            // expect a number for enum value error if not
                            if compiler.tk != Some(Token::Num) { panic!("{}: bad enum initializer", compiler.line); }
                            i = compiler.ival;
                            compiler.next();
                        }
                        // add enum constant to symbol table with its value and type INT
                        compiler.sym.push(Ident { tk: Token::Num, hash: 0, name, class: Token::Num, ty: Type::INT, val: i, hclass: Token::Num, hty: Type::INT, hval: 0 });
                        i += 1; // bump value for next constant if not set
                        if compiler.tk == Some(Token::Comma) { compiler.next(); } // skip the commas
                    }
                    // move past the }
                    compiler.next();
                }
            }
            // parse global variable declarations like int x, y
            while compiler.tk != Some(Token::Semicolon) && compiler.tk != Some(Token::CurlyClose) {
                // start with base type int or char for this variable
                let mut ty = bt;
                // check for * tokens to make pointer types like int *p
                while compiler.tk == Some(Token::Mul) {
                    compiler.next();
                    ty = match ty { Type::CHAR => Type::PTR, Type::INT => Type::PTR, Type::PTR => Type::PTR };
                }
                // expect an identifier for variable name error if not
                if compiler.tk != Some(Token::Id) { panic!("{}: bad global declaration", compiler.line); }
                // grab variable name from source code
                let name = compiler.src[compiler.p - 1..compiler.p].to_string();
                // check symbol table for duplicate names to avoid redefinition
                let id_idx = compiler.sym.iter().position(|id| id.name == name);
                if id_idx.is_some() { panic!("{}: duplicate global definition", compiler.line); }
                // move past identifier
                compiler.next();
                // add global variable to symbol table with address in the data segment
                let id = Ident { tk: Token::Id, hash: 0, name, class: Token::Glo, ty, val: compiler.data.len() as Int, hclass: Token::Glo, hty: ty, hval: 0 };
                compiler.sym.push(id);
                // allocate space in the data segment for variable and init to 0
                compiler.data.extend(&[0; std::mem::size_of::<Int>()]);
                // if comma expect another variable name and keep going
                if compiler.tk == Some(Token::Comma) { compiler.next(); }
            }
            // move to next token after the declaration
            compiler.next();
        }
    
        // run the compiled bytecode and get the exit code
        let exit_code = compiler.run();
        // exit the program with the exit code to signal success or failure
        std::process::exit(exit_code);
    }
