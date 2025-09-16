init_database:
    mbslave init --create-database --empty
    mbslave psql -f psql -f updates/schema-change/30.all.sql
    echo 'UPDATE replication_control SET current_schema_sequence = 30;' | mbslave psql
    mbslave auto-import

init_index:
    cargo run --release -- init
