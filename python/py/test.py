import ese_parser
import unittest, datetime
import platform

from datetime import datetime
from platform import system

class TestEseDbMethods(unittest.TestCase):
	def test_test_db(self):
		edb = ese_parser.PyEseDb()
		edb.load("../lib/testdata/test.edb")
		tables = edb.get_tables()
		self.assertEqual(tables, ['MSysObjects', 'MSysObjectsShadow', 'MSysObjids', 'MSysLocales', 'TestTable'])
		t = "TestTable"
		tbl = edb.open_table(t)
		self.assertTrue(tbl > 0)

		self.assertEqual(len(edb.get_columns(t)), 18)
		
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "Bit")), 0)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "UnsignedByte")), 255)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "Short")), None)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "Long")), -2147483648)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "Currency")), 350050)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "IEEESingle")), 3.141592025756836)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "IEEEDouble")), 3.141592653589)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "UnsignedLong")), 4294967295)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "LongLong")), 9223372036854775807)
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "UnsignedShort")), 65535)
		self.assertEqual(datetime.utcfromtimestamp(edb.get_row(tbl, edb.get_column(t, "DateTime"))), datetime(2021, 3, 29, 11, 49, 47))
		self.assertEqual(edb.get_row(tbl, edb.get_column(t, "GUID")), "{4D36E96E-E325-11CE-BFC1-08002BE10318}")

		b = edb.get_row(tbl, edb.get_column(t, "Binary"))
		ind = 0
		for i in b:
			self.assertEqual(i, ind % 255)
			ind += 1
		
		b = edb.get_row(tbl, edb.get_column(t, "LongBinary"))
		ind = 0
		for i in b:
			self.assertEqual(i, ind % 255)
			ind += 1

		abc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890";

		b = edb.get_row(tbl, edb.get_column(t, "Text"))
		ind = 0
		for i in b:
			self.assertEqual(i, abc[ind % len(abc)])
			ind += 1

		b = edb.get_row_mv(tbl, edb.get_column(t, "Text"), 2)
		h = "Hello"
		ind = 0
		for i in range(0,len(b)-2):
			self.assertEqual(b[i], ord(h[ind]))
			ind += 1

		if system() == 'Windows':
			b = edb.get_row(tbl, edb.get_column(t, "LongText"))
			ind = 0
			for i in b:
				self.assertEqual(i, abc[ind % len(abc)])
				ind += 1

		edb.close_table(tbl)

	def test_datetimes(self):
		edb = ese_parser.PyEseDb()
		edb.load("../lib/testdata/Current.mdb")
		t = "CLIENTS"
		tbl = edb.open_table(t)
		d1 = edb.get_row(tbl, edb.get_column(t, "InsertDate"))
		# Wrong format. Negative value due to bad data for timestamp (column header),
		# i.e. -2209161600 is converted to 1899-12-30 etc.
		self.assertEqual(datetime.utcfromtimestamp(d1), datetime(1899, 12, 30, 0, 0))
		# Move to the row with actual timestamp
		d2 = edb.get_row_mv(tbl, edb.get_column(t, "InsertDate"), 2)
		self.assertEqual(ese_parser.wrap_date_time_from_filetime(d2), "2021-06-12 23:47:21.232323500 UTC")
		

if __name__ == '__main__':
    unittest.main()
