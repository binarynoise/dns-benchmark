@file:OptIn(ExperimentalSerializationApi::class)

import java.io.File
import kotlin.system.exitProcess
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.MissingFieldException
import kotlinx.serialization.Serializable
import kotlinx.serialization.hocon.Hocon
import kotlinx.serialization.hocon.decodeFromConfig
import kotlinx.serialization.hocon.encodeToConfig
import com.typesafe.config.ConfigFactory
import com.typesafe.config.ConfigRenderOptions

private val configRenderOptions = ConfigRenderOptions.defaults().setComments(true).setOriginComments(false).setJson(false)

@Serializable
data class Config(
    /**
     * The list of DNS-Servers to test. IPv4 or IPv6
     */
    val servers: List<String>,
    /**
     * The domain to query.
     * Will be formatted with two ints:
     * a random prefix and the iteration count to generate unique requests
     */
    val domain: String,
    /**
     * The command to run.
     * Will be formatted with two strings:
     * the queried domain and the dns server.
     * Example: `"drill %s @%s"`
     */
    val command: String,
    /**
     * The regex used to extract the time
     */
    val timeRegex: String,
    /**
     * How often each DNS server is queried
     */
    val repeat: Int,
    /**
     * The time to wait between queries to a server
     */
    val delay: Duration,
) {
    companion object {
        fun load(): Config {
            val file = File("application.conf")
            if (!file.exists()) {
                file.writeText(Config(emptyList(), "", "", "", 100, 1.seconds).toHocon())
                printErr("Config not found, created empty: $file")
                exitProcess(1)
            }
            try {
                val config = ConfigFactory.parseFile(file)
                return Hocon.decodeFromConfig(config)
            } catch (e: MissingFieldException) {
                printErr("Config incomplete, please add ${e.missingFields.joinToString()}")
                exitProcess(1)
            }
        }
    }
    
    fun toHocon(): String {
        return Hocon.encodeToConfig(this).root().render(configRenderOptions)
    }
}

fun printErr(msg: Any) = System.err.println(msg)
fun printErr(msg: String) = System.err.println(msg)
