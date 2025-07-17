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

## Create `CommandLineInterface.java`

Next step is creating `CommandLineInterface.java` file in the `/src/main/java/com/codingame/gameengine/runner` folder.

The source code of the new file:

```java
package com.codingame.gameengine.runner;

import com.codingame.gameengine.runner.dto.GameResultDto;
import com.google.common.io.Files;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Options;
import org.apache.commons.io.FileUtils;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.Charset;
import java.nio.file.Paths;
import java.util.List;
import java.util.Properties;

public class CommandLineInterface {

    public static void main(String[] args) {
        try {
            Options options = new Options();

            // Define required options
            options.addOption("h", false, "Print the help")
                    .addOption("p1", true, "Player 1 command line.")
                    .addOption("p2", true, "Player 2 command line.")
                    .addOption("p3", true, "Player 3 command line.")
                    .addOption("p4", true, "Player 4 command line.")
                    .addOption("p5", true, "Player 5 command line.")
                    .addOption("p6", true, "Player 6 command line.")
                    .addOption("p7", true, "Player 7 command line.")
                    .addOption("p8", true, "Player 8 command line.")
                    .addOption("league", true, "League level")
                    .addOption("s", false, "Server mode")
                    .addOption("r", true, "File input for replay")
                    .addOption("l", true, "File output for logs")
                    .addOption("seed", true, "Seed");

            CommandLine cmd = new DefaultParser().parse(options, args);

            // Rendering the replay
            if (cmd.hasOption("r")) {
                File json = new File(cmd.getOptionValue("r"));
                String jsonResult = FileUtils.readFileToString(json);
                new Renderer(8888).render(2, jsonResult);
                return;
            }

            // Launch Game
            MultiplayerGameRunner gameRunner = new MultiplayerGameRunner();

            // Choose league level, depends on the game (19 is max allowed by the engine)
            Integer leagueLevel = Integer.parseInt(cmd.getOptionValue("league", "19"));
            gameRunner.setLeagueLevel(leagueLevel);

            if (cmd.hasOption("seed")) {
                gameRunner.setSeed(Long.valueOf(cmd.getOptionValue("seed")));
            } else {
                gameRunner.setSeed(System.currentTimeMillis());
            }

            GameResultDto result = gameRunner.gameResult;

            int playerCount = 0;

            // CG supports multiplayer games up to 8 players
            for (int i = 1; i <= 8; ++i) {
                if (cmd.hasOption("p" + i)) {
                    gameRunner.addAgent(cmd.getOptionValue("p" + i), cmd.getOptionValue("p" + i));
                    playerCount += 1;
                }
            }

            if (cmd.hasOption("s")) {
                gameRunner.start();
            } else {
                Method initialize = GameRunner.class.getDeclaredMethod("initialize", Properties.class);
                initialize.setAccessible(true);
                initialize.invoke(gameRunner, new Properties());

                Method runAgents = GameRunner.class.getDeclaredMethod("runAgents");
                runAgents.setAccessible(true);
                runAgents.invoke(gameRunner);

                if (cmd.hasOption("l")) {
                    Method getJSONResult = GameRunner.class.getDeclaredMethod("getJSONResult");
                    getJSONResult.setAccessible(true);

                    Files.asCharSink(Paths.get(cmd.getOptionValue("l")).toFile(), Charset.defaultCharset())
                            .write((String) getJSONResult.invoke(gameRunner));
                }

                for (int i = 0; i < playerCount; ++i) {
                    System.out.println(result.scores.get(i));
                }

                for (String line : result.uinput) {
                    System.out.println(line);
                }
            }

            // We have to clean players process properly
            Field getPlayers = GameRunner.class.getDeclaredField("players");
            getPlayers.setAccessible(true);
            @SuppressWarnings("unchecked")
            List<Agent> players = (List<Agent>) getPlayers.get(gameRunner);

            if (players != null) {
                for (Agent player : players) {
                    Field getProcess = CommandLinePlayerAgent.class.getDeclaredField("process");
                    getProcess.setAccessible(true);
                    Process process = (Process) getProcess.get(player);

                    process.destroy();
                }
            }
        } catch (Exception e) {
            System.err.println(e);
            e.printStackTrace(System.err);
            System.exit(1);
        }
    }

}
```

## Creating .jar file

Now you can create `.jar` file using the following command:

```
mvn package
```

This would create multiple `.jar` files in the `target` folder, just copy `winter-2024-sprawl-1.0-SNAPSHOT.jar` to your arena folder and rename `.jar` file to `referee.jar` for simplicity.