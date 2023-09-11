import kotlin.random.Random

@Suppress("UNREACHABLE_CODE", "KotlinRedundantDiagnosticSuppress")
fun main() {
    val servers = arrayOf(
        "1.1.1.1", "1.0.0.1", "2606:4700:4700::1111", "2606:4700:4700::1001", // Cloudflare
        "9.9.9.9", "149.112.112.112", "2620:fe::fe", "2620:fe::9", // Quad9
        "193.110.81.0", "185.253.5.0", "2a0f:fc80::", "2a0f:fc81::", // dns0.eu
        "8.8.8.8", "8.8.4.4", "2001:4860:4860::8888", "2001:4860:4860::8844", // Google
//        "62.109.121.1", "62.109.121.2", "2a01:c30::530", "2a01:c30::531", // O2
//        "79.143.183.251", "79.143.183.252", "213.136.95.10", "213.136.95.11", // Contabo
        "76.76.2.5", "76.76.10.5", "2606:1a40::5", "2606:1a40:1::5", // ControlD
    )

//    println(servers.joinToString("\n"))
    
    val results = servers.associateWith { mutableListOf<Int>() }
    
    val prefix = (1..9).joinToString("") { Random.nextInt(10).toString() }
    
    val command: (String, String) -> String = { server: String, domain: String -> "drill $domain @$server" }
    val regex = Regex(";; Query time: (\\d+) msec")
    
    val runtime = Runtime.getRuntime()
    
    repeat(100) {
        val domain = "${prefix}${it}.binarynoise.de"
        
        for ((server, result) in results) {
            val process = runtime.exec(command(server, domain))
            val exitCode = process.waitFor()
            
            if (exitCode != 0) {
                result.add(1000)
                continue
            }
            
            val out = process.inputReader().readLines()
            val err = process.errorReader().readLines()
            
            check(err.isEmpty())
            
            val line = out.find { it.contains("Query time") }
            check(line != null)
            val match = regex.find(line)
            check(match != null)
            
            val time = match.groups[1]!!.value.toInt()
            result.add(time)
        }
    }
    
    val table: List<List<String>> = results.entries.map<Map.Entry<String, List<Int>>, List<String>> { (server, results) ->
        results.mapTo(mutableListOf(server)) { it.toString() }
    }.transpose()
    
    println(table.joinToString("\n") { it.joinToString(",") })
}

/**
 * Given a list of lists (ie a matrix), transpose it
 */
fun <T> List<List<T>>.transpose(): List<List<T>> {
    return (this[0].indices).map { i -> (this.indices).map { j -> this[j][i] } }
}

/**
 * Given an array of arrays (ie a matrix), transpose it
 */
inline fun <reified T> Array<Array<T>>.transpose(): Array<Array<T>> {
    return Array(this[0].size) { i -> Array(this.size) { j -> this[j][i] } }
}
