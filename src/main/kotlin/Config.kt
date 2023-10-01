@file:OptIn(ExperimentalSerializationApi::class)

import java.io.File
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
    val domain: DomainConfig,
    val runs: Map<String, RunConfig>,
    val delay: Duration,
) {
    companion object {
        private val DEFAULT
            get() = Config(
                DomainConfig.DEFAULT,
                mapOf("example" to RunConfig.DEFAULT),
                1.seconds,
            )
        
        fun load(): Config {
            val file = File("application.conf")
            check(file.exists()) {
                file.writeText(Config.DEFAULT.toHocon())
                "Config not found, created empty: $file"
            }
            try {
                val config = ConfigFactory.parseFile(file).resolve()
                return Hocon.decodeFromConfig(config)
            } catch (e: MissingFieldException) {
                throw InvalidConfigException("Config incomplete, please add ${e.missingFields.joinToString()}", e)
            }
        }
    }
    
    fun toHocon(): String {
        return Hocon.encodeToConfig(this).root().render(configRenderOptions)
    }
}

@Serializable
data class DomainConfig(
    val file: String? = null,
    val fixed: String? = null,
    val formatted: String? = null,
    val repeat: Int = 1,
) {
    companion object {
        val DEFAULT
            get() = DomainConfig(
                file = "/path/to/domains-file.txt",
                fixed = "example.com",
                formatted = "%d%d.example.com",
                repeat = 1,
            )
    }
}

@Serializable
data class RunConfig(
    val command: String,
    val timeRegex: String,
    val servers: List<String>,
) {
    companion object {
        val DEFAULT
            get() = RunConfig(
                command = "dig %s @%s",
                timeRegex = ";; Query time: (\\d+) msec",
                servers = listOf("1.1.1.1"),
            )
    }
}

class InvalidConfigException(message: String, cause: Throwable? = null) : IllegalArgumentException(message, cause)
