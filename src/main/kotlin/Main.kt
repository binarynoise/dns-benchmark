import java.io.File
import java.lang.management.ManagementFactory
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import kotlin.random.Random

val runtime: Runtime = Runtime.getRuntime()

@Suppress("UNREACHABLE_CODE", "KotlinRedundantDiagnosticSuppress")
fun main() {
    println("dns-benchmark")
    val timestamp = LocalDateTime.now().format(DateTimeFormatter.ISO_LOCAL_DATE_TIME)
    
    val debug = ManagementFactory.getRuntimeMXBean().inputArguments.any { it.contains("jdwp") }
    
    val config = Config.load()
    
    val domains = buildList {
        when {
            config.domain.fixed != null -> {
                repeat(config.domain.repeat) { add(config.domain.fixed) }
            }
            config.domain.formatted != null -> {
                val prefix = Random.nextInt(from = 0, until = Int.MAX_VALUE)
                repeat(config.domain.repeat) { add(config.domain.formatted.format(prefix, it)) }
            }
            config.domain.file != null -> {
                val file = File(config.domain.file)
                
                if (!file.exists()) throw InvalidConfigException("Domain file doesn't exist: ${config.domain.file}")
                
                file.readLines().let { lines ->
                    if (lines.isEmpty()) throw InvalidConfigException("Domain file is empty: $config.domainFile")
                    
                    repeat(config.domain.repeat) { addAll(lines) }
                }
            }
            else -> throw InvalidConfigException("No domain(s) defined")
        }
    }
    println("Loaded ${config.runs.size} runs and ${domains.size} domains")
    
    config.runs.forEach { (name, run) ->
        val results = run.servers.associateWith { mutableListOf<Int>() }
        val regex = Regex(run.timeRegex)
        
        println("Run $name: benchmarking ${run.servers.size} servers")
        domains.forEach { domain ->
            print(".")
            
            for ((server, result) in results) {
                val process = runtime.exec(run.command.format(domain, server))
                val exitCode = process.waitFor()
                val out = process.inputStream.bufferedReader().readLines()
                val err = process.errorStream.bufferedReader().readLines()
                
                try {
                    check(exitCode == 0) {
                        buildString {
                            appendLine("Command failed: ${run.command.format(domain, server)}")
                            appendLine("Exit code: $exitCode")
                            appendOutAndErr(out, err)
                        }
                    }
                    
                    val line = out.findLast { regex.matches(it) }
                    check(line != null) {
                        buildString {
                            appendLine("Command failed: ${run.command.format(domain, server)}")
                            appendLine("time not found")
                            appendOutAndErr(out, err)
                        }
                    }
                    val match = regex.find(line)
                    check(match != null) {
                        buildString {
                            appendLine("Command failed: ${run.command.format(domain, server)}")
                            appendLine("time not found")
                            appendOutAndErr(out, err)
                        }
                    }
                    
                    val time = match.groupValues[1].toInt()
                    result.add(time)
                } catch (e: IllegalStateException) {
                    if (debug) throw e
                    val time = 10000
                    result.add(time)
                }
            }
            Thread.sleep(config.delay.inWholeMilliseconds)
        }
        println()
        
        val table: List<List<String>> = results.entries.map<Map.Entry<String, List<Int>>, List<String>> { (server, results) ->
            results.mapTo(mutableListOf("\"" + server + "\"")) { it.toString() }
        }.transpose()
        
        val output = File("dns-benchmark-$timestamp-$name.csv")
        output.writeText(table.joinToString("\n") { it.joinToString(",") })
        
        println("Run $name: done")
    }
}

private fun StringBuilder.appendOutAndErr(out: List<String>, err: List<String>) {
    appendLine("Output:")
    out.forEach(::appendLine)
    appendLine()
    appendLine("Error:")
    err.forEach(::appendLine)
}

/**
 * Given a list of lists (ie a matrix), transpose it
 */
fun <T> List<List<T>>.transpose(): List<List<T>> {
    if (this.isEmpty()) return emptyList()
    return (this[0].indices).map { i -> (this.indices).map { j -> this[j][i] } }
}

/**
 * Given an array of arrays (ie a matrix), transpose it
 */
inline fun <reified T> Array<Array<T>>.transpose(): Array<Array<T>> {
    if (this.isEmpty()) return emptyArray()
    return Array(this[0].size) { i -> Array(this.size) { j -> this[j][i] } }
}
