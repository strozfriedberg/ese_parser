# ese_parser library
## Extensible Storage Engine (ESE) Database File (EDB) parser library

What's supported:
- reading db file headers
- reading page headers
- page tags loading
- root page header loading
- the catalog (data type) definition loading (columns)
- table page values (rows)
- multi-valued sparse columns
- default values
- tagged data (un)compression

This library implements the `ese_trait` trait, which provides the following features:
- load database
- open/close table
- get list of tables
- get list of columns
- get column in current row by types (get_column_str, get_column_dyn, get_column_dyn_varlen)
- get column multi value column (get_column_dyn_mv)
- move row (first, next, prev, last)

An example program, `ese_parser`, is included in the project. This executable will dump all (or selected) tables from an ESE database to the console.

When compiled with the `nt_comparison` feature for Windows (`cargo build --example ese_parser --features nt_comparison`), this program has three modes:
* `EseParser` - accesses database using this parser
* `EseApi` - accesses database using MS esent.dll
* `Both` - parses using both methods, compares the results, and reports any differences.
```
C:> ese_parser.exe /help
[/m mode] [/t table] db path
where mode is one of [EseAPI, EseParser, *Both - default]
```
There are a couple of ways to run `ese_parser`:
* Directly with `cargo run`
  * `cargo run --example ese_parser /m eseparser testdata/decompress_test.edb`
  * `cargo run --example ese_parser --features nt_comparison /m both testdata/decompress_test.edb`
  * Note that this will fail because the `nt_comparison` feature was not enabled: `cargo run --example ese_parser /m both testdata/decompress_test.edb`
* Building and running ese_parser
  * `cargo build --example ese_parser`
    * `./target/debug/examples/ese_parser /m eseparser testdata/decompress_test.edb `
  * `cargo build --example ese_parser --features nt_comparison`
    * `./target/debug/examples/ese_parser /m both testdata/decompress_test.edb`

To ensure that the unit tests for all sub-projects are run, make sure to specify `--all-targets` when running cargo test: `cargo test --all-targets`.

### [EDB format  specification](https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc)
### [Open Source Microsoft ESE reader](https://github.com/microsoft/Extensible-Storage-Engine)