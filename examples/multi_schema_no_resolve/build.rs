use std::fs::File;
use std::io::Write;

use xsd_parser::{
    config::{
        Config, Generate, GeneratorFlags, IdentQuadruple, InterpreterFlags, NamespaceIdent,
        OptimizerFlags, ParserFlags, Schema,
    },
    generate, Error,
};
use xsd_parser_types::misc::Namespace;

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=common_types.xsd");
    println!("cargo:rerun-if-changed=request_v1.xsd");
    println!("cargo:rerun-if-changed=request_v2.xsd");

    // All schemas are provided as explicit inputs, so the parser does not
    // need to resolve `xs:import` against the file system. We deliberately
    // do NOT enable RESOLVE_INCLUDES — that's the configuration that
    // triggers the panic this example reproduces.
    let mut config = Config::default().with_quick_xml();

    config.parser.flags = ParserFlags::DEFAULT_NAMESPACES;
    config.parser.schemas = vec![
        Schema::File("common_types.xsd".into()),
        Schema::File("request_v1.xsd".into()),
        Schema::File("request_v2.xsd".into()),
    ];

    config.interpreter.flags = InterpreterFlags::all() - InterpreterFlags::WITH_NUM_BIG_INT;
    config.optimizer.flags = OptimizerFlags::all();
    config.generator.flags = GeneratorFlags::all();

    config.generator.generate = Generate::Types(vec![
        IdentQuadruple {
            ns: Some(NamespaceIdent::Namespace(Namespace(
                b"http://example.com/request/01".to_vec().into(),
            ))),
            schema: None,
            name: "Request".to_string(),
            type_: xsd_parser::models::IdentType::Element,
        },
        IdentQuadruple {
            ns: Some(NamespaceIdent::Namespace(Namespace(
                b"http://example.com/request/02".to_vec().into(),
            ))),
            schema: None,
            name: "Request".to_string(),
            type_: xsd_parser::models::IdentType::Element,
        },
    ]);

    let code = generate(config)?;
    let code = code.to_string();

    let mut file = File::create("src/generated.rs")?;
    file.write_all(code.as_bytes())?;

    Ok(())
}
