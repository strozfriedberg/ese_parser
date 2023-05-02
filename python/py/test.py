import ese_parser
import unittest
import platform

from datetime import datetime
from platform import system

class TestEseDbMethods(unittest.TestCase):
	def test_test_db_path(self):
		edb = ese_parser.PyEseDb("../lib/testdata/test.edb")
		self._test_db(edb)

	def test_test_db_file(self):
		f = open("../lib/testdata/test.edb", "rb")
		edb = ese_parser.PyEseDb(f)
		self._test_db(edb)
		f.close()

	def _test_db(self, edb):
		tables = edb.get_tables()
		self.assertEqual(tables, ['MSysObjects', 'MSysObjectsShadow', 'MSysObjids', 'MSysLocales', 'TestTable'])
		t = "TestTable"
		tbl = edb.open_table(t)
		self.assertTrue(tbl > 0)

		self.assertEqual(len(edb.get_columns(t)), 18)

		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "Bit")), 0)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "UnsignedByte")), 255)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "Short")), None)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "Long")), -2147483648)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "Currency")), 350050)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "IEEESingle")), 3.141592025756836)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "IEEEDouble")), 3.141592653589)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "UnsignedLong")), 4294967295)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "LongLong")), 9223372036854775807)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "UnsignedShort")), 65535)
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "DateTime")), datetime(2021, 3, 29, 11, 49, 47))
		self.assertEqual(edb.get_value(tbl, edb.get_column(t, "GUID")), "{4D36E96E-E325-11CE-BFC1-08002BE10318}")

		b = edb.get_value(tbl, edb.get_column(t, "Binary"))
		ind = 0
		for i in b:
			self.assertEqual(i, ind % 255)
			ind += 1

		b = edb.get_value(tbl, edb.get_column(t, "LongBinary"))
		ind = 0
		for i in b:
			self.assertEqual(i, ind % 255)
			ind += 1

		abc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890";

		b = edb.get_value(tbl, edb.get_column(t, "Text"))
		ind = 0
		for i in b:
			self.assertEqual(i, abc[ind % len(abc)])
			ind += 1

		b = edb.get_value_mv(tbl, edb.get_column(t, "Text"), 2)
		h = "Hello"
		ind = 0
		for i in range(0,len(b)-2):
			self.assertEqual(b[i], ord(h[ind]))
			ind += 1

		if system() == 'Windows':
			b = edb.get_value(tbl, edb.get_column(t, "LongText"))
			ind = 0
			for i in b:
				self.assertEqual(i, abc[ind % len(abc)])
				ind += 1

		edb.close_table(tbl)

	def test_datetimes(self):
		edb = ese_parser.PyEseDb("../lib/testdata/Current.mdb")
		t = "CLIENTS"
		tbl = edb.open_table(t)
		d1 = edb.get_value(tbl, edb.get_column(t, "InsertDate"))
		self.assertEqual(d1, datetime(2021, 6, 12, 23, 47, 21, 232324))
		edb.move_row(tbl, 1)
		d2 = edb.get_value(tbl, edb.get_column(t, "InsertDate"))
		self.assertEqual(d2, datetime(2021, 6, 12, 23, 48, 45, 468902))
		edb.move_row(tbl, 1)
		d3 = edb.get_value(tbl, edb.get_column(t, "InsertDate"))
		self.assertEqual(d3, datetime(2021, 6, 12, 23, 49, 44, 255548))
		edb.move_row(tbl, 2147483647)  # move to the last row
		d_last = edb.get_value(tbl, edb.get_column(t, "InsertDate"))
		self.assertEqual(d_last, datetime(2021, 6, 20, 20, 48, 48, 366867))

		# Move to the first row again
		edb.move_row(tbl, -2147483648)
		total_access = edb.get_value(tbl, edb.get_column(t, "TotalAccesses"))  # for this column type get_value fetches data fine
		self.assertEqual(total_access, 310)
		edb.move_row(tbl, 1)  # move to the second row
		total_access = edb.get_value(tbl, edb.get_column(t, "TotalAccesses"))
		self.assertEqual(total_access, 101)

		edb.close_table(tbl)

if __name__ == '__main__':
    unittest.main()
