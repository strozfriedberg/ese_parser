# ese_parser library
# Extensible Storage Engine (ESE) Database File (EDB) parser library
# [EDB format  specification](https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc)

What's supported:
- reading db file header
- reading page(s) header
- page tags loading
- root page header loading
- the catalog (data type) definition loading (columns)
- table page values (rows)
- multi-valued sparse columns
- default values
- tagged data (un)compression

This library implement `ese_trait` trait, that give such features:
- load database
- open/close table
- get list of tables
- get list of columns
- get column in current row by types (get_column_str, get_column_dyn, get_column_dyn_varlen)
- get column multi value column (get_column_dyn_mv)
- move row (first, next, prev, last)