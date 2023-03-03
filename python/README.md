# Python wrapper of ese_parser

Exports all `ese_trait` interfaces with python functions.

To build wheel, run `build_wheel.bat`.

Available methods:
- `load(str) -> Option<String>` - load database, return String in case of error
- `open_table(table_str) -> u64` - open table, return table ID
- `close_table(table_id) -> bool` - close table, return true on success
- `get_tables() -> Vec<String>` - get array of tables
- `get_column(table_str, column_name_str) -> PyColumnInfo`
- `get_columns(table_str) -> Vec<PyColumnInfo>` - where `PyColumnInfo` is a structure with get methods:
```
struct PyColumnInfo {
    name: String,
    id: u32,
    typ: u32,
    cbmax: u32,
    cp: u16
}
```
- `move_row(table_id, crow) -> bool` - where `crow` is one of:
 ```
JET_MoveFirst = 2147483648;
JET_MovePrevious = -1;
JET_MoveNext = 1;
JET_MoveLast = 2147483647;
 ```
 - `get_row(table_id, column_info) -> Option<PyObject>` - will return object or None, if field is NULL
 - `get_row_mv(table_id, column_info, multi_value_index) -> PyResult<Option<PyObject>>` - will return multi-value object at index (itagSequence) or None, if field is NULL

Python wrapper usage sample:
```
import ese_parser

edb = ese_parser.PyEseDb()
edb.load("../lib/testdata/test.edb")
tables = edb.get_tables()
print("tables: {}".format(tables))

for t in tables:
	tbl = edb.open_table(t)

	print("table {} opened: {}".format(t, tbl))
	columns = edb.get_columns(t)

	for c in columns:
		print("name: {}, id: {}, type: {}, cbmax: {}, cp: {}".format(c.name, c.id, c.typ, c.cbmax, c.cp))

	while True:
		print("|", end='')
		for c in columns:
			i = edb.get_row(tbl, c)
			print(" {} |".format(i), end='')
		print("")
		if not edb.move_row(tbl, 1):
			break
	edb.close_table(tbl)

```