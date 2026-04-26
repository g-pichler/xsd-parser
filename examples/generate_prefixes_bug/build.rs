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
    println!("cargo:rerun-if-changed=request_v1.xsd");
    println!("cargo:rerun-if-changed=request_v2.xsd");
    println!("cargo:rerun-if-changed=request_v3.xsd");

    // Both request schemas declare the same prefix `xmlns:rq` for
    // *different* target namespaces. With `GENERATE_PREFIXES`, the
    // parser is supposed to invent a fresh prefix for the second
    // namespace so each one can land in its own module.
    //
    // BUG: in `parser/mod.rs::determine_prefixes`, the
    // `GENERATE_PREFIXES` branch inserts the generated prefix into
    // `known_prefixes` but never assigns it back to `info.prefix`.
    // As a result, the second namespace's `info.prefix` stays `None`,
    // its `ModuleMeta.prefix` is `None`, and its types collapse into
    // the root module — clashing with the first schema's `Request`
    // type.
    //
    // No `xs:import` declarations are needed to surface this bug, so
    // `RESOLVE_INCLUDES` is left off and the panic exercised by
    // `multi_schema_no_resolve` does not apply here.
    let mut config = Config::default().with_quick_xml();

    config.parser.flags = ParserFlags::DEFAULT_NAMESPACES
        | ParserFlags::ALTERNATIVE_PREFIXES
        | ParserFlags::GENERATE_PREFIXES;
    config.parser.schemas = vec![
        Schema::File("request_v1.xsd".into()),
        Schema::File("request_v2.xsd".into()),
        Schema::File("request_v3.xsd".into()),
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
        IdentQuadruple {
            ns: Some(NamespaceIdent::Namespace(Namespace(
                b"http://example.com/request/03".to_vec().into(),
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
