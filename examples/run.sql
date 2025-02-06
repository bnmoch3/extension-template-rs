load './build/debug/hello.duckdb_extension';
select * from hello('Alice', count = 10);
