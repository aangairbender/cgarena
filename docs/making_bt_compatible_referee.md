# How to make a brutaltester-compatible referee

Let's use [CodinGame's Winter Challenge 2024](https://www.codingame.com/contests/winter-challenge-2024) as an example.

## Prerequisites 

You would need Java 17 (newer versions should work too) and Maven installed.

## Cloning the referee

The first step is to clone the official referee.

```
git clone https://github.com/CodinGame/WinterChallenge2024-Cellularena.git
cd WinterChallenge2024-Cellularena
```

## Modifying `pom.xml`

You need to make few modifictaions to `pom.xml`.

Add a new dependency to `<dependencies>` section:

```xml
<dependencies>
    ...

    <dependency>
        <groupId>commons-cli</groupId>
        <artifactId>commons-cli</artifactId>
        <version>1.3.1</version>
    </dependency>
</dependencies>
```

Then add a new section inside `<project>` below `<dependencies>`:

```xml
<project>
    ...

    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.3</version>
                <configuration>
                    <source>${maven.compiler.source}</source>
                    <target>${maven.compiler.target}</target>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-jar-plugin</artifactId>
                <version>3.4.2</version>
                <configuration>
                    <archive>
                        <manifest>
                            <addClasspath>true</addClasspath>
                            <mainClass>com.codingame.gameengine.runner.CommandLineInterface</mainClass>
                        </manifest>
                    </archive>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-shade-plugin</artifactId>
                <version>3.6.0</version>
                <executions>
                    <execution>
                        <phase>package</phase>
                        <goals>
                            <goal>shade</goal>
                        </goals>
                        <configuration>
                            <transformers>
                                <transformer implementation="org.apache.maven.plugins.shade.resource.ManifestResourceTransformer">
                                    <mainClass>com.codingame.gameengine.runner.CommandLineInterface</mainClass>
                                </transformer>
                            </transformers>
                        </configuration>
                    </execution>
                </executions>
            </plugin>
        </plugins>
    </build>
</project>
```


The full version would look like this :

```xml
<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
	xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
	<modelVersion>4.0.0</modelVersion>

	<groupId>com.codingame.game</groupId>
	<artifactId>winter-2024-sprawl</artifactId>
	<version>1.0-SNAPSHOT</version>

    <properties>
        <gamengine.version>4.5.0</gamengine.version>
        <java.version>17</java.version>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
    </properties>

    <dependencies>
        <dependency>
            <groupId>com.codingame.gameengine</groupId>
            <artifactId>core</artifactId>
            <version>${gamengine.version}</version>
        </dependency>

        <dependency>
            <groupId>com.codingame.gameengine</groupId>
            <artifactId>runner</artifactId>
            <version>${gamengine.version}</version>
        </dependency>
        
        <dependency>
            <groupId>com.codingame.gameengine</groupId>
            <artifactId>module-endscreen</artifactId>
            <version>${gamengine.version}</version>
        </dependency>

        <dependency>
            <groupId>com.codingame.gameengine</groupId>
            <artifactId>module-entities</artifactId>
            <version>${gamengine.version}</version>
        </dependency>
        
        <dependency>
            <groupId>commons-cli</groupId>
            <artifactId>commons-cli</artifactId>
            <version>1.3.1</version>
        </dependency>
    </dependencies>

     <build>
		<plugins>

			<plugin>
				<groupId>org.apache.maven.plugins</groupId>
				<artifactId>maven-compiler-plugin</artifactId>
				<version>3.3</version>
				<configuration>
					<source>${maven.compiler.source}</source>
					<target>${maven.compiler.target}</target>
				</configuration>
			</plugin>

			<plugin>
				<groupId>org.apache.maven.plugins</groupId>
				<artifactId>maven-jar-plugin</artifactId>
				<version>3.4.2</version>
				<configuration>
					<archive>
						<manifest>
							<addClasspath>true</addClasspath>
							<mainClass>com.codingame.gameengine.runner.CommandLineInterface</mainClass>
						</manifest>
					</archive>
				</configuration>
			</plugin>

			<plugin>
				<groupId>org.apache.maven.plugins</groupId>
				<artifactId>maven-shade-plugin</artifactId>
				<version>3.6.0</version>
				<executions>
					<execution>
						<phase>package</phase>
						<goals>
							<goal>shade</goal>
						</goals>
						<configuration>
							<transformers>
								<transformer
									implementation="org.apache.maven.plugins.shade.resource.ManifestResourceTransformer">
									<mainClass>com.codingame.gameengine.runner.CommandLineInterface</mainClass>
								</transformer>
							</transformers>
						</configuration>
					</execution>
				</executions>
			</plugin>

		</plugins>
	</build>
</project>
```

## Install the CG Arena command-line interface

`cgarena init` generates the maintained referee patch as `CommandLineInterface.java`.
Copy that file into the referee source tree:

```sh
cp ../CommandLineInterface.java \
  referee/src/main/java/com/codingame/gameengine/runner/CommandLineInterface.java
```

The interface defines the match and replay contract:

- `-p1` through `-p8`: complete player command lines
- `-seed`: signed 64-bit match seed
- `-league`: league level, default `19`
- `-l`: replay JSON output path
- `-r`: replay JSON input path
- `-port`: loopback replay renderer port, default `8888`

Match mode writes the JSON artifact requested by `-l`, prints scores and referee input to
stdout, prints failures to stderr, and exits nonzero on failure. Different processes may use
the same seed safely when their `-l` paths differ.

Replay mode reads the complete CodinGame replay JSON, derives participant count from its
`agents` array, starts `Renderer` on `-port`, then prints its renderer URL and
`Exposed web server dir: <path>`. `watch_replay.py` consumes the directory line, copies the
static bundle, terminates the renderer, and reaps it.

The generated `pom_build_section.xml` is the maintained Maven build configuration. Replace
the referee POM's existing `<build>` section with its contents. Keep the `commons-cli`
dependency shown above. The shade configuration owns the stable output name and main class;
do not maintain a second copy of either configuration in the referee repository.

## Creating .jar file

Build the shaded JAR:

```sh
mvn package
```

The maintained Maven configuration writes `target/referee.jar`. Keep the referee repository
at `<arena>/referee`, so the generated Python launchers find
`<arena>/referee/target/referee.jar`.

Verify a deterministic two-player match outside CG Arena:

```sh
java --add-opens java.base/java.lang=ALL-UNNAMED \
  -jar target/referee.jar \
  -p1 "/absolute/path/to/bot" -p2 "/absolute/path/to/bot" \
  -seed 123 -league 19 -l /tmp/cgarena-replay-123.json
```

Success exits `0`, writes the requested JSON file, and writes referee output to stdout.
Invalid player commands, unwritable output, or referee failures write a stack trace to stderr
and exit nonzero. Parallel same-seed matches are supported when each process receives a
different `-l` path.

Verify replay generation:

```sh
java --add-opens java.base/java.lang=ALL-UNNAMED \
  -jar target/referee.jar \
  -r /tmp/cgarena-replay-123.json -port 8888
```

Read the printed HTTP URL in a browser. Stop the renderer with `Ctrl+C`. Invalid/missing JSON
or an occupied port is written to stderr and exits nonzero. The replay JSON's `agents` array
is the source of participant count for two-player and multiplayer artifacts.

The integration is supported on macOS, Linux, and Windows with Python 3.9 or newer, Java 17
or newer, and Maven for referee builds. Use platform-native path syntax and the installed
Python launcher (`python3` on macOS/Linux, commonly `py -3` on Windows).