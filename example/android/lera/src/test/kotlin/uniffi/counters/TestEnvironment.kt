package uniffi.counters

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths

internal object TestEnvironment {
    init {
        configureNativeLibrary()
    }

    fun ensure() = Unit

    private fun configureNativeLibrary() {
        val osName = System.getProperty("os.name")?.lowercase() ?: ""
        val libraryExtension = when {
            osName.contains("mac") -> "dylib"
            osName.contains("win") -> "dll"
            else -> "so"
        }

        val targetRoot = Paths.get("..", "..", "rust", "target").toAbsolutePath().normalize()
        val candidate = Files.walk(targetRoot).use { paths ->
            paths
                .filter { Files.isRegularFile(it) && it.fileName.toString() == "libcounters.$libraryExtension" }
                .findFirst()
                .orElseThrow {
                    IllegalStateException("Unable to locate libcounters.$libraryExtension under $targetRoot")
                }
        }

        System.setProperty("uniffi.component.counters.libraryOverride", candidate.toString())
    }
}
