# ese_parser of 
# Extensible Storage Engine (ESE) Database File (EDB) parser

This is helper (test) tool to dump all (or selected) tables data from EDB files.

```
C:> ese_parser.exe /help
[/m mode] [/t table] db path
where mode one of [EseAPI, EseParser, *Both - default]
```

There is 3 modes to use:
- EseAPI: access database using MS esent.dll
- EseParser: access database using our own parser
- Both: test mode, that first calling EseAPI interface, then EseParser and comparing returned results, reporting about any differences.