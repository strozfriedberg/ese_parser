import ese_parser
from datetime import datetime

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
			if c.typ == 8: # datetime
				i = datetime.utcfromtimestamp(i)
			print(" {} |".format(i), end='')
		print("")
		if not edb.move_row(tbl, 1):
			break
	edb.close_table(tbl)
