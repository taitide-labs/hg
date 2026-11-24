package com.taitidelabs.hg.player

import kotlin.math.abs
import kotlin.math.max

object HgVideoPlayerProtocol {
    const val COMPONENT_NAME = "hg-video-player"

    const val PROP_SRC = "src"
    const val PROP_PAUSED = "paused"
    const val PROP_RATE = "rate"
    const val PROP_VOLUME = "volume"
    const val PROP_MUTED = "muted"
    const val PROP_SEEK_SECS = "seek_secs"
    const val PROP_SEEK_SEQ = "seek_seq"
    const val PROP_ASPECT = "aspect"

    val ALL_PROP_KEYS = listOf(
        PROP_SRC,
        PROP_PAUSED,
        PROP_RATE,
        PROP_VOLUME,
        PROP_MUTED,
        PROP_SEEK_SECS,
        PROP_SEEK_SEQ,
        PROP_ASPECT,
    )

    const val EVENT_PROGRESS = "progress"
    const val EVENT_ENDED = "ended"
    const val EVENT_ERROR = "error"

    val ALL_EVENT_NAMES = listOf(
        EVENT_PROGRESS,
        EVENT_ENDED,
        EVENT_ERROR,
    )

    const val PAYLOAD_POSITION_SECS = "position_secs"
    const val PAYLOAD_DURATION_SECS = "duration_secs"
    const val PAYLOAD_MESSAGE = "message"

    const val DEFAULT_RATE = 1.0
    const val MIN_RATE = 0.25
    const val MAX_RATE = 4.0

    const val DEFAULT_VOLUME = 0.8
    const val MIN_VOLUME = 0.0
    const val MAX_VOLUME = 1.0

    const val PROGRESS_INTERVAL_MS = 500L

    private const val DOUBLE_EPSILON = 2.220446049250313e-16
    private const val UINT_MAX = 4_294_967_295L

    fun progressPayload(positionSecs: Long, durationSecs: Long): Map<String, Long> = mapOf(
        PAYLOAD_POSITION_SECS to positionSecs.coerceIn(0L, UINT_MAX),
        PAYLOAD_DURATION_SECS to durationSecs.coerceIn(0L, UINT_MAX),
    )

    fun errorPayload(message: String): Map<String, String> = mapOf(
        PAYLOAD_MESSAGE to message,
    )

    fun computeTransform(
        viewWidth: Int,
        viewHeight: Int,
        videoWidth: Int,
        videoHeight: Int,
        aspect: VideoAspect,
    ): TextureTransform {
        if (viewWidth <= 0 || viewHeight <= 0 || videoWidth <= 0 || videoHeight <= 0) {
            return TextureTransform(1f, 1f, 0f, 0f)
        }

        val viewRatio = viewWidth.toFloat() / viewHeight.toFloat()
        val videoRatio = videoWidth.toFloat() / videoHeight.toFloat()

        val (scaleX, scaleY) = when (aspect) {
            VideoAspect.ORIGINAL -> fitInside(viewRatio, videoRatio)
            VideoAspect.FILL -> fillOutside(viewRatio, videoRatio)
            VideoAspect.CINEMA_16_9 -> fitInside(viewRatio, 16f / 9f)
            VideoAspect.PORTRAIT_9_16 -> fitInside(viewRatio, 9f / 16f)
        }

        return TextureTransform(
            scaleX = scaleX,
            scaleY = scaleY,
            pivotX = viewWidth / 2f,
            pivotY = viewHeight / 2f,
        )
    }

    private fun fitInside(viewRatio: Float, targetRatio: Float): Pair<Float, Float> {
        return if (viewRatio > targetRatio) {
            (targetRatio / viewRatio) to 1f
        } else if (viewRatio < targetRatio) {
            1f to (viewRatio / targetRatio)
        } else {
            1f to 1f
        }
    }

    private fun fillOutside(viewRatio: Float, targetRatio: Float): Pair<Float, Float> {
        return if (viewRatio > targetRatio) {
            1f to (viewRatio / targetRatio)
        } else if (viewRatio < targetRatio) {
            (targetRatio / viewRatio) to 1f
        } else {
            1f to 1f
        }
    }

    internal fun parseNonNegativeU32(value: Any?): Long {
        val raw = when (value) {
            is Byte -> value.toLong()
            is Short -> value.toLong()
            is Int -> value.toLong()
            is Long -> value
            else -> return 0L
        }
        return if (raw < 0L) 0L else raw.coerceAtMost(UINT_MAX)
    }

    internal fun parseNonNegativeU64(value: Any?): Long {
        val raw = when (value) {
            is Byte -> value.toLong()
            is Short -> value.toLong()
            is Int -> value.toLong()
            is Long -> value
            else -> return 0L
        }
        return max(0L, raw)
    }

    internal fun doublesEqual(a: Double, b: Double): Boolean = abs(a - b) < DOUBLE_EPSILON
}

enum class VideoAspect(val protocolValue: String) {
    ORIGINAL("original"),
    FILL("fill"),
    CINEMA_16_9("16:9"),
    PORTRAIT_9_16("9:16"),
    ;

    companion object {
        fun parse(raw: String?): VideoAspect = when (raw) {
            FILL.protocolValue -> FILL
            CINEMA_16_9.protocolValue -> CINEMA_16_9
            PORTRAIT_9_16.protocolValue -> PORTRAIT_9_16
            else -> ORIGINAL
        }
    }
}

data class TextureTransform(
    val scaleX: Float,
    val scaleY: Float,
    val pivotX: Float,
    val pivotY: Float,
)

sealed interface PropChange {
    data object Unchanged : PropChange
    data object Source : PropChange
    data object Paused : PropChange
    data object Rate : PropChange
    data object Volume : PropChange
    data object Muted : PropChange
    data class Seek(val positionSecs: Long) : PropChange
    data object Aspect : PropChange
}

class VideoPlayerProps {
    var src: String? = null
        private set
    var paused: Boolean = false
        private set
    var rate: Double = HgVideoPlayerProtocol.DEFAULT_RATE
        private set
    var volume: Double = HgVideoPlayerProtocol.DEFAULT_VOLUME
        private set
    var muted: Boolean = false
        private set
    var seekSecs: Long = 0L
        private set
    var seekSeq: Long = 0L
        private set
    var aspect: VideoAspect = VideoAspect.ORIGINAL
        private set

    fun effectiveVolume(): Float = if (muted) 0f else volume.toFloat()

    fun applyProp(name: String, value: Any?): PropChange = when (name) {
        HgVideoPlayerProtocol.PROP_SRC -> applySrc(value)
        HgVideoPlayerProtocol.PROP_PAUSED -> applyPaused(value)
        HgVideoPlayerProtocol.PROP_RATE -> applyRate(value)
        HgVideoPlayerProtocol.PROP_VOLUME -> applyVolume(value)
        HgVideoPlayerProtocol.PROP_MUTED -> applyMuted(value)
        HgVideoPlayerProtocol.PROP_SEEK_SECS -> applySeekSecs(value)
        HgVideoPlayerProtocol.PROP_SEEK_SEQ -> applySeekSeq(value)
        HgVideoPlayerProtocol.PROP_ASPECT -> applyAspect(value)
        else -> PropChange.Unchanged
    }

    private fun applySrc(value: Any?): PropChange {
        val next = (value as? String)?.takeIf { it.isNotEmpty() }
        return if (src == next) {
            PropChange.Unchanged
        } else {
            src = next
            PropChange.Source
        }
    }

    private fun applyPaused(value: Any?): PropChange {
        val next = (value as? Boolean) ?: false
        return if (paused == next) {
            PropChange.Unchanged
        } else {
            paused = next
            PropChange.Paused
        }
    }

    private fun applyRate(value: Any?): PropChange {
        val parsed = (value as? Number)?.toDouble()?.takeIf { it.isFinite() }
            ?: HgVideoPlayerProtocol.DEFAULT_RATE
        val next = parsed.coerceIn(HgVideoPlayerProtocol.MIN_RATE, HgVideoPlayerProtocol.MAX_RATE)
        return if (HgVideoPlayerProtocol.doublesEqual(rate, next)) {
            PropChange.Unchanged
        } else {
            rate = next
            PropChange.Rate
        }
    }

    private fun applyVolume(value: Any?): PropChange {
        val parsed = (value as? Number)?.toDouble()?.takeIf { it.isFinite() }
            ?: HgVideoPlayerProtocol.DEFAULT_VOLUME
        val next = parsed.coerceIn(HgVideoPlayerProtocol.MIN_VOLUME, HgVideoPlayerProtocol.MAX_VOLUME)
        return if (HgVideoPlayerProtocol.doublesEqual(volume, next)) {
            PropChange.Unchanged
        } else {
            volume = next
            PropChange.Volume
        }
    }

    private fun applyMuted(value: Any?): PropChange {
        val next = (value as? Boolean) ?: false
        return if (muted == next) {
            PropChange.Unchanged
        } else {
            muted = next
            PropChange.Muted
        }
    }

    private fun applySeekSecs(value: Any?): PropChange {
        seekSecs = HgVideoPlayerProtocol.parseNonNegativeU32(value)
        return PropChange.Unchanged
    }

    private fun applySeekSeq(value: Any?): PropChange {
        val next = HgVideoPlayerProtocol.parseNonNegativeU64(value)
        return if (next == 0L || next == seekSeq) {
            seekSeq = next
            PropChange.Unchanged
        } else {
            seekSeq = next
            PropChange.Seek(seekSecs)
        }
    }

    private fun applyAspect(value: Any?): PropChange {
        val next = VideoAspect.parse(value as? String)
        return if (aspect == next) {
            PropChange.Unchanged
        } else {
            aspect = next
            PropChange.Aspect
        }
    }
}
