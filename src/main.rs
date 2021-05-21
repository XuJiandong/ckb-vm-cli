mod cost_model;
mod debugger;

use bytes::Bytes;
use ckb_vm::machine::asm::{AsmCoreMachine, AsmMachine};
use ckb_vm::machine::SupportMachine;
use ckb_vm::DefaultMachineBuilder;
use std::fs::File;
use std::io::Read;
use std::process::exit;

fn main() {
    use clap::{App, Arg};
    let matches = App::new("ckb-vm-cli")
        .version("0.1")
        .about("A command line tool for CKB VM")
        .arg(
            Arg::with_name("bin")
                .long("bin")
                .short("b")
                .value_name("filename")
                .help("Specify the name of the executable")
                .required(true),
        )
        .arg(Arg::with_name("args").multiple(true))
        .get_matches();

    let args: Vec<String> = matches
        .values_of("args")
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.clone().into())
        .collect();

    let bin_path = matches.value_of("bin").unwrap();
    let mut file = File::open(bin_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let buffer: Bytes = buffer.into();
    let mut args2: Vec<Bytes> = vec![Bytes::copy_from_slice(bin_path.as_ref())];
    let args_tail: Vec<Bytes> = args.into_iter().map(|s| s.into()).collect();
    args2.extend(args_tail);

    let asm_core = AsmCoreMachine::new_with_max_cycles(1 << 31);
    let core = DefaultMachineBuilder::<Box<AsmCoreMachine>>::new(asm_core)
        .instruction_cycle_func(Box::new(cost_model::instruction_cycles))
        .syscall(Box::new(debugger::Debugger::new()))
        .build();
    let mut machine = AsmMachine::new(core, None);

    machine.load_program(&buffer, &args2).unwrap();
    let result = machine.run();
    let cycles = machine.machine.cycles();
    let c: f64 = cycles as f64;
    if cycles > 1000000 {
        println!("Cycles = {:.2} M cycles", c / 1000. / 1000.);
    } else {
        println!("Cycles = {:.2} K cycles (It's below 1 M)", c / 1000.);
    }
    if result != Ok(0) {
        println!("Error result: {:?}", result);
        exit(i32::from(result.unwrap_or(-99)));
    }
}
