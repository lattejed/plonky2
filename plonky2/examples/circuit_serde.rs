use std::fs;

use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData};
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2::util::gate_serialization::default::DefaultGateSerializer;
use plonky2::util::generator_serialization::default::DefaultGeneratorSerializer;
#[cfg(test)]
use tempfile;

/// Serialize / Deserialize the Fibonacci example circuit.
fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let count = 10_000;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    // The arithmetic circuit.
    let initial_a = builder.add_virtual_target();
    let initial_b = builder.add_virtual_target();
    let mut prev_target = initial_a;
    let mut cur_target = initial_b;
    for _ in 0..count - 1 {
        let temp = builder.add(prev_target, cur_target);
        prev_target = cur_target;
        cur_target = temp;
    }

    // Public inputs are the two initial values (provided below) and the result (which is generated).
    builder.register_public_input(initial_a);
    builder.register_public_input(initial_b);
    builder.register_public_input(cur_target);

    // Provide initial values.
    let mut pw = PartialWitness::new();
    pw.set_target(initial_a, F::ZERO);
    pw.set_target(initial_b, F::ONE);

    // Ciruit, proof & verification
    let circuit_data = builder.build::<C>();
    let proof = circuit_data.prove(pw.clone())?;
    println!(
        "{}th Fibonacci number mod |F| (starting with {}, {}) is: {} (in memory circuit)",
        count, proof.public_inputs[0], proof.public_inputs[1], proof.public_inputs[2]
    );
    circuit_data.verify(proof)?;

    // Serialize circuit
    let gate_serializer = DefaultGateSerializer;
    // TODO: Add a ctor for this
    let generator_serializer = DefaultGeneratorSerializer {
        _phantom: std::marker::PhantomData::<C>,
    };

    let circuit_data_bytes = circuit_data
        .to_bytes(&gate_serializer, &generator_serializer)
        .map_err(|_| anyhow::Error::msg("CircuitData serialization failed."))?;

    // TODO:
    #[derive(serde::Serialize, serde::Deserialize)]
    struct SerializedCircuitData {
        circuit_data: Vec<u8>,
    }

    let serialized_circuit_data = SerializedCircuitData {
        circuit_data: circuit_data_bytes,
    };
    let json_str = serde_json::to_string(&serialized_circuit_data)?;

    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_file = temp_file.path();

    fs::write(temp_file, json_str)?;
    let json_bytes = fs::read(temp_file)?;
    let serialized_circuit_data: SerializedCircuitData = serde_json::from_slice(&json_bytes)?;

    let circuit_data_from_bytes = CircuitData::<F, C, D>::from_bytes(
        &serialized_circuit_data.circuit_data,
        &gate_serializer,
        &generator_serializer,
    )
    .map_err(|_| anyhow::Error::msg("CommonCircuitData deserialization failed."))?;

    assert_eq!(&circuit_data, &circuit_data_from_bytes);

    let proof = circuit_data_from_bytes.prove(pw)?;
    println!(
        "{}th Fibonacci number mod |F| (starting with {}, {}) is: {} (deserialized circuit)",
        count, proof.public_inputs[0], proof.public_inputs[1], proof.public_inputs[2]
    );
    circuit_data_from_bytes.verify(proof)?;

    Ok(())
}
