package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FaceReviewAnalyzerTest {
    @Test
    fun cropExpandsFaceBoundsAndClampsAtTopLeft() {
        val crop = faceCropBounds(0, 0, 100, 100, imageWidth = 300, imageHeight = 400)

        assertEquals(FaceCropBounds(0, 0, 116, 116), crop)
    }

    @Test
    fun cropClampsAtBottomRight() {
        val crop = faceCropBounds(200, 300, 300, 400, imageWidth = 300, imageHeight = 400)

        assertEquals(FaceCropBounds(184, 284, 300, 400), crop)
    }

    @Test
    fun cropRejectsInvalidAndTooSmallBounds() {
        assertNull(faceCropBounds(20, 20, 20, 80, imageWidth = 200, imageHeight = 200))
        assertNull(faceCropBounds(30, 30, 40, 40, imageWidth = 200, imageHeight = 200, marginRatio = 0f))
        assertNull(faceCropBounds(0, 0, 40, 40, imageWidth = 0, imageHeight = 200))
    }

    @Test
    fun cropKeepsPartiallyOutOfFrameBoundsInsideImage() {
        val crop = faceCropBounds(-12, 40, 62, 120, imageWidth = 80, imageHeight = 140)

        assertEquals(FaceCropBounds(0, 27, 72, 133), crop)
    }
}
