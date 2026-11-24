package com.taitidelabs.hg.player

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class HgVideoPlayerProtocolTest {
    @Test
    fun protocolConstantsMatchRustContract() {
        assertEquals("hg-video-player", HgVideoPlayerProtocol.COMPONENT_NAME)
        assertEquals(
            listOf(
                "src",
                "paused",
                "rate",
                "volume",
                "muted",
                "seek_secs",
                "seek_seq",
                "aspect",
            ),
            HgVideoPlayerProtocol.ALL_PROP_KEYS,
        )
        assertEquals(
            listOf("progress", "ended", "error"),
            HgVideoPlayerProtocol.ALL_EVENT_NAMES,
        )
        assertEquals(3, HgVideoPlayerPool.DEFAULT_CAPACITY)
        assertEquals(500L, HgVideoPlayerProtocol.PROGRESS_INTERVAL_MS)
    }

    @Test
    fun videoAspectParsesAndSerializesRoundtrip() {
        assertEquals(VideoAspect.ORIGINAL, VideoAspect.parse("original"))
        assertEquals(VideoAspect.FILL, VideoAspect.parse("fill"))
        assertEquals(VideoAspect.CINEMA_16_9, VideoAspect.parse("16:9"))
        assertEquals(VideoAspect.PORTRAIT_9_16, VideoAspect.parse("9:16"))
        assertEquals(VideoAspect.ORIGINAL, VideoAspect.parse("unknown"))
        assertEquals(VideoAspect.ORIGINAL, VideoAspect.parse(null))

        assertEquals("original", VideoAspect.ORIGINAL.protocolValue)
        assertEquals("fill", VideoAspect.FILL.protocolValue)
        assertEquals("16:9", VideoAspect.CINEMA_16_9.protocolValue)
        assertEquals("9:16", VideoAspect.PORTRAIT_9_16.protocolValue)
    }

    @Test
    fun propsApplyStateMachineClampingAndSeekSeqDedup() {
        val props = VideoPlayerProps()
        assertEquals(0.8, props.volume, 1e-6)
        assertEquals(1.0, props.rate, 1e-6)
        assertFalse(props.paused)
        assertFalse(props.muted)
        assertNull(props.src)

        assertEquals(
            PropChange.Source,
            props.applyProp("src", "http://127.0.0.1:8888/online/1/index.m3u8"),
        )
        assertEquals(
            PropChange.Unchanged,
            props.applyProp("src", "http://127.0.0.1:8888/online/1/index.m3u8"),
        )

        assertEquals(PropChange.Paused, props.applyProp("paused", true))
        assertEquals(PropChange.Rate, props.applyProp("rate", 8.0))
        assertEquals(4.0, props.rate, 1e-6)
        assertEquals(PropChange.Rate, props.applyProp("rate", 0.1))
        assertEquals(0.25, props.rate, 1e-6)
        assertEquals(PropChange.Rate, props.applyProp("rate", Double.NaN))
        assertEquals(1.0, props.rate, 1e-6)

        assertEquals(PropChange.Volume, props.applyProp("volume", 1.5))
        assertEquals(1.0, props.volume, 1e-6)
        assertEquals(PropChange.Volume, props.applyProp("volume", -0.5))
        assertEquals(0.0, props.volume, 1e-6)
        assertEquals(PropChange.Volume, props.applyProp("volume", 0.6))
        assertEquals(0.6, props.volume, 1e-6)

        assertEquals(PropChange.Muted, props.applyProp("muted", true))
        assertEquals(0f, props.effectiveVolume(), 1e-6f)
        assertEquals(PropChange.Muted, props.applyProp("muted", false))
        assertEquals(0.6f, props.effectiveVolume(), 1e-6f)

        assertEquals(PropChange.Unchanged, props.applyProp("seek_secs", 42))
        assertEquals(PropChange.Unchanged, props.applyProp("seek_seq", 0))
        assertEquals(PropChange.Seek(42L), props.applyProp("seek_seq", 1))
        assertEquals(PropChange.Unchanged, props.applyProp("seek_seq", 1))
        assertEquals(PropChange.Unchanged, props.applyProp("seek_secs", 99))
        assertEquals(PropChange.Seek(99L), props.applyProp("seek_seq", 2))

        assertEquals(PropChange.Aspect, props.applyProp("aspect", "9:16"))
        assertEquals(VideoAspect.PORTRAIT_9_16, props.aspect)

        assertEquals(PropChange.Source, props.applyProp("src", ""))
        assertNull(props.src)
    }

    @Test
    fun eventPayloadsMatchContract() {
        val progress = HgVideoPlayerProtocol.progressPayload(18L, 90L)
        assertEquals(18L, progress["position_secs"])
        assertEquals(90L, progress["duration_secs"])

        val error = HgVideoPlayerProtocol.errorPayload("decode failed")
        assertEquals("decode failed", error["message"])
    }

    @Test
    fun computeTransformMaintainsExpectedAspectRatios() {
        val original = HgVideoPlayerProtocol.computeTransform(
            viewWidth = 1080,
            viewHeight = 1920,
            videoWidth = 1920,
            videoHeight = 1080,
            aspect = VideoAspect.ORIGINAL,
        )
        assertEquals(1f, original.scaleX, 1e-4f)
        assertEquals((1080f / 1920f) / (1920f / 1080f), original.scaleY, 1e-4f)
        assertEquals(540f, original.pivotX, 1e-4f)
        assertEquals(960f, original.pivotY, 1e-4f)

        val fill = HgVideoPlayerProtocol.computeTransform(
            viewWidth = 1080,
            viewHeight = 1920,
            videoWidth = 1920,
            videoHeight = 1080,
            aspect = VideoAspect.FILL,
        )
        assertEquals((1920f / 1080f) / (1080f / 1920f), fill.scaleX, 1e-4f)
        assertEquals(1f, fill.scaleY, 1e-4f)

        val portraitInLandscape = HgVideoPlayerProtocol.computeTransform(
            viewWidth = 1920,
            viewHeight = 1080,
            videoWidth = 1920,
            videoHeight = 1080,
            aspect = VideoAspect.PORTRAIT_9_16,
        )
        assertEquals((9f / 16f) / (1920f / 1080f), portraitInLandscape.scaleX, 1e-4f)
        assertEquals(1f, portraitInLandscape.scaleY, 1e-4f)

        val invalid = HgVideoPlayerProtocol.computeTransform(0, 1080, 1920, 1080, VideoAspect.ORIGINAL)
        assertEquals(TextureTransform(1f, 1f, 0f, 0f), invalid)
    }

    private class DummyPlayer(val id: Int)

    @Test
    fun reusablePoolHonorsCapacityResetAndClearAll() {
        var nextId = 0
        val resetCounts = mutableMapOf<Int, Int>()
        val disposed = mutableListOf<Int>()

        val pool = ReusablePool(
            maxCapacity = 2,
            create = {
                nextId += 1
                DummyPlayer(nextId)
            },
            reset = { player -> resetCounts[player.id] = (resetCounts[player.id] ?: 0) + 1 },
            dispose = { player -> disposed.add(player.id) },
        )

        val p1 = pool.acquire()
        val p2 = pool.acquire()
        val p3 = pool.acquire()
        assertEquals(listOf(1, 2, 3), listOf(p1.id, p2.id, p3.id))

        assertTrue(pool.release(p1))
        assertTrue(pool.release(p2))
        assertFalse(pool.release(p3))
        assertEquals(2, pool.idleCount)
        assertEquals(listOf(3), disposed)

        val reused = pool.acquire()
        assertSame(p2, reused)
        assertEquals(1, pool.idleCount)
        assertTrue((resetCounts[p2.id] ?: 0) >= 3)

        pool.clearAll()
        assertEquals(0, pool.idleCount)
        assertEquals(listOf(3, 1), disposed)
    }
}
