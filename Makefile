
watch_test: src/*
	echo "$?" | tr " " '\n' | entr cargo test
	
watch_build: src/*
	echo "$?" | tr " " '\n' | entr make build

build:
	clear
	cargo build --message-format short

watch: src/*
	
	echo "$?" | tr " " '\n' | entr cargo run
