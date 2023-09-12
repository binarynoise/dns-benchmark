import kotlin.random.Random

@Suppress("UNREACHABLE_CODE", "KotlinRedundantDiagnosticSuppress")
fun main() {
    println("dns-benchmark")
    
    val config = Config.load()
    print("benchmarking ${config.servers.size} servers")
    
    val results = config.servers.associateWith { mutableListOf<Int>() }
    val prefix = Random.nextInt(from = 0, until = Int.MAX_VALUE)
    val regex = Regex(config.timeRegex)
    val runtime = Runtime.getRuntime()
    
    repeat(config.repeat) {
        print(".")
        val domain = config.domain.format(prefix, it)
        
        for ((server, result) in results) {
            val process = runtime.exec(config.command.format(domain, server))
            val exitCode = process.waitFor()
            
            if (exitCode != 0) {
                result.add(1000)
                continue
            }
            
            val out = process.inputStream.bufferedReader().readLines()
            val err = process.errorStream.bufferedReader().readLines()
            check(err.isEmpty())
            
            val line = out.findLast { regex.matches(it) }
            check(line != null)
            val match = regex.find(line)
            check(match != null)
            
            val time = match.groupValues[1].toInt()
            result.add(time)
        }
        Thread.sleep(config.delay.inWholeMilliseconds)
    }
    
    repeat(3) { println() }
    
    val table: List<List<String>> = results.entries.map<Map.Entry<String, List<Int>>, List<String>> { (server, results) ->
        results.mapTo(mutableListOf(server)) { it.toString() }
    }.transpose()
    
    println(table.joinToString("\n") { it.joinToString(",") })
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
