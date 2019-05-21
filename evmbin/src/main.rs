// based on evmbin from parity-ethereum


// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.



extern crate cita_vm;

extern crate rustc_hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate docopt;
use docopt::Docopt;

extern crate ethereum_types;

extern crate parity_bytes as bytes;
use bytes::Bytes;
use rustc_hex::FromHex;


//use cita_vm;
//use cita_vm::json_tests::common::*;
//use evm::extmock;
//use cita_vm::evm::extmock;

//use std::io;
//use std::io::Write;

use std::fmt;
use std::fs;

use ethereum_types::{Address, U256};
use std::cell::RefCell;
use std::sync::Arc;

use std::time::{Instant};



const USAGE: &'static str = r#"
Usage:
    cita-evm [options]
    cita-evm [-h | --help]

Transaction options:
    --code-file CODEFILE    Read contract code from file as hex (without 0x).
    --code CODE        Contract code as hex (without 0x).
    --to ADDRESS       Recipient address (without 0x).
    --from ADDRESS     Sender address (without 0x).
    --input DATA       Input data as hex (without 0x).
    --expected DATA    Expected return data as hex (without 0x).
    --gas GAS          Supplied gas as hex (without 0x).
    --gas-price WEI    Supplied gas price as hex (without 0x).

    -h, --help         Display this message and exit.
"#;

fn main() {
	//panic_hook::set_abort();
	//env_logger::init();

	let args: Args = Docopt::new(USAGE).and_then(|d| d.deserialize()).unwrap_or_else(|e| e.exit());

	run_call(args)
}

const START_GAS: u64 = 100000000;

fn run_call(args: Args) {
	let _from = arg(args.from(), "--from");
	let _to = arg(args.to(), "--to");
	let code_file = arg(args.code_file(), "--code-file");
	let code = arg(args.code(), "--code");
	let _gas = arg(args.gas(), "--gas");
	let _gas_price = arg(args.gas_price(), "--gas-price");
	let calldata = arg(args.data(), "--input");
    let expected = arg(args.expected(), "--expected");

	if code.is_none() && code_file.is_none() {
		die("Either --code or --code-file is required.");
	}

    if expected.is_none() {
        die("Expected return data --expected is required.");
    }

    let code = code_file.unwrap();
    let input_data = calldata.unwrap().clone();
    let expected_return = expected.unwrap().clone();

    // do a test run and check that return value equals expected


    env_logger::init();
    let db =  Arc::new(cita_vm::state::MemoryDB::new(false));
    let mut state = cita_vm::state::State::new(db).unwrap();
    /*
    let code = "6080604052600436106049576000357c0100000000000000000000000000000\
                000000000000000000000000000900463ffffffff16806360fe47b114604e57\
                80636d4ce63c146078575b600080fd5b348015605957600080fd5b506076600\
                4803603810190808035906020019092919050505060a0565b005b3480156083\
                57600080fd5b50608a60aa565b6040518082815260200191505060405180910\
                390f35b8060008190555050565b600080549050905600a165627a7a72305820\
                99c66a25d59f0aa78f7ebc40748fa1d1fbc335d8d780f284841b30e0365acd9\
                60029";
    */
    state.new_contract(
        &Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1"),
        U256::from(10),
        U256::from(1),
        //hex::decode(code).unwrap(),
        code.clone(),
    );

    state.new_contract(
        &Address::from("0x1000000000000000000000000000000000000000"),
        U256::from(1_000_000_000_000_000u64),
        U256::from(1),
        vec![],
    );

    let block_data_provider: Arc<cita_vm::BlockDataProvider> =
        Arc::new(cita_vm::BlockDataProviderMock::default());
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    //let config = cita_vm::Config::default();
    let config = cita_vm::Config {
        block_gas_limit: 100000000,
        check_nonce: true,
    };

    let tx = cita_vm::Transaction {
        from: Address::from("0x1000000000000000000000000000000000000000"),
        to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")),
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: START_GAS,
        gas_price: U256::from(1),
        //input: hex::decode("60fe47b1000000000000000000000000000000000000000000000000000000000000002a").unwrap(),
        input: input_data.clone(),
    };

    let r = cita_vm::exec(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    )
    .unwrap();
    // r: (output, gas_left, logs)
    //println!("return={:?}", r);

    match r {
        cita_vm::evm::InterpreterResult::Normal(data, gas_left, _logs) => {
            let return_data = hex::encode(data);
            if return_data != expected_return {
                println!("got: {:?}   expected: {:?}", return_data, expected_return);
                panic!("wrong return!!");
            }
            println!("gas_used: {:?}", START_GAS - gas_left);
        },
        cita_vm::evm::InterpreterResult::Revert(_data, _gas_left) => {
            println!("revert.");
        },
        cita_vm::evm::InterpreterResult::Create(_data, _gas_left, _logs, _address) => {
            println!("create.");
        },
    }


    // now run in loop...

    let iterations = 100;
    let mut total_duration = std::time::Duration::new(0, 0);

    for _i in 0..iterations {

        let db =  Arc::new(cita_vm::state::MemoryDB::new(false));
        let mut state = cita_vm::state::State::new(db).unwrap();

        state.new_contract(
            &Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1"),
            U256::from(10),
            U256::from(1),
            code.clone(),
        );
        state.new_contract(
            &Address::from("0x1000000000000000000000000000000000000000"),
            U256::from(1_000_000_000_000_000u64),
            U256::from(1),
            vec![],
        );

        let state_data_provider = Arc::new(RefCell::new(state));

        let tx = cita_vm::Transaction {
            from: Address::from("0x1000000000000000000000000000000000000000"),
            to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")),
            value: U256::from(0),
            nonce: U256::from(1),
            gas_limit: 50000000,
            gas_price: U256::from(1),
            input: input_data.clone(),
        };

        let start_run = Instant::now();

        let _r = cita_vm::exec(
            block_data_provider.clone(),
            state_data_provider.clone(),
            context.clone(),
            config.clone(),
            tx,
        )
        .unwrap();

        let run_duration = start_run.elapsed();
        total_duration = total_duration + run_duration;

    }

    let avg_duration = total_duration / iterations;
    println!("code avg run time: {:?}", avg_duration);

}




#[derive(Debug, Deserialize)]
struct Args {
    flag_code_file: Option<String>,
	flag_only: Option<String>,
	flag_from: Option<String>,
	flag_to: Option<String>,
	flag_code: Option<String>,
	flag_gas: Option<String>,
	flag_gas_price: Option<String>,
	flag_input: Option<String>,
    flag_expected: Option<String>,
}

impl Args {
	pub fn gas(&self) -> Result<U256, String> {
		match self.flag_gas {
			Some(ref gas) => gas.parse().map_err(to_string),
			None => Ok(U256::from(u64::max_value())),
		}
	}

	pub fn gas_price(&self) -> Result<U256, String> {
		match self.flag_gas_price {
			Some(ref gas_price) => gas_price.parse().map_err(to_string),
			None => Ok(U256::zero()),
		}
	}

	pub fn from(&self) -> Result<Address, String> {
		match self.flag_from {
			Some(ref from) => from.parse().map_err(to_string),
			None => Ok(Address::default()),
		}
	}

	pub fn to(&self) -> Result<Address, String> {
		match self.flag_to {
			Some(ref to) => to.parse().map_err(to_string),
			None => Ok(Address::default()),
		}
	}

	pub fn code(&self) -> Result<Option<Bytes>, String> {
		match self.flag_code {
			Some(ref code) => code.from_hex().map(Some).map_err(to_string),
			None => Ok(None),
		}
	}

	pub fn data(&self) -> Result<Option<Bytes>, String> {
		match self.flag_input {
			Some(ref input) => input.from_hex().map_err(to_string).map(Some),
			None => Ok(None),
		}
	}

	pub fn expected(&self) -> Result<Option<String>, String> {
		match self.flag_expected {
			Some(ref expected) => expected.parse().map_err(to_string).map(Some),
			None => Ok(None),
		}
	}

    pub fn code_file(&self) -> Result<Option<Bytes>, String> {
        match self.flag_code_file {
            Some(ref filename) => {
                let code_hex = fs::read_to_string(filename).unwrap();
                println!("code_hex length: {:?}", code_hex.len());
                code_hex.from_hex().map_err(to_string).map(Some)
            },
            None => Ok(None),
        }
    }

}

fn arg<T>(v: Result<T, String>, param: &str) -> T {
	v.unwrap_or_else(|e| die(format!("Invalid {}: {}", param, e)))
}

fn to_string<T: fmt::Display>(msg: T) -> String {
	format!("{}", msg)
}

fn die<T: fmt::Display>(msg: T) -> ! {
	println!("{}", msg);
	::std::process::exit(-1)
}

#[cfg(test)]
mod tests {
	use docopt::Docopt;
	use super::{Args, USAGE};

	fn run<T: AsRef<str>>(args: &[T]) -> Args {
		Docopt::new(USAGE).and_then(|d| d.argv(args.into_iter()).deserialize()).unwrap()
	}

	#[test]
	fn should_parse_all_the_options() {
		let args = run(&[
			"cita-evm",
			"--gas", "1",
			"--gas-price", "2",
			"--from", "0000000000000000000000000000000000000003",
			"--to", "0000000000000000000000000000000000000004",
			"--code", "05",
			"--input", "06",
		]);

		assert_eq!(args.gas(), Ok(1.into()));
		assert_eq!(args.gas_price(), Ok(2.into()));
		assert_eq!(args.from(), Ok(3.into()));
		assert_eq!(args.to(), Ok(4.into()));
		assert_eq!(args.code(), Ok(Some(vec![05])));
		assert_eq!(args.data(), Ok(Some(vec![06])));
	}

}
