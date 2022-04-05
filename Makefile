all: java-dependencies
.PHONY: java-dependencies

java-dependencies: target/java-dependencies.jar
target/java-dependencies.jar:
	cd java-dependencies $(MAKE) shadowjar
	cp java-dependencies/build/libs/java-dependencies-all.jar target/java-dependencies.jar
