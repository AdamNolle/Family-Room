package com.familyroom.app

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertTrue
import org.junit.Test

class RoomPaletteContrastTest {
    @Test
    fun lightAndDarkTextPairsMeetNormalTextContrast() {
        for (dark in listOf(false, true)) {
            val scheme = roomColorScheme(dark)
            for (pair in roomTextContrastPairs(scheme)) {
                val ratio = contrastRatio(pair.foreground, pair.background)
                val mode = if (dark) "Dark" else "Light"
                assertTrue(
                    mode + " " + pair.label + " contrast is " + String.format("%.2f", ratio) +
                        ":1; expected at least 4.5:1",
                    ratio >= 4.5,
                )
            }
        }
    }

    private fun contrastRatio(foreground: Color, background: Color): Double {
        val foregroundLuminance = luminance(foreground)
        val backgroundLuminance = luminance(background)
        val lighter = maxOf(foregroundLuminance, backgroundLuminance)
        val darker = minOf(foregroundLuminance, backgroundLuminance)
        return (lighter + 0.05) / (darker + 0.05)
    }

    private fun luminance(color: Color): Double =
        0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)

    private fun linear(channel: Float): Double {
        val value = channel.toDouble()
        return if (value <= 0.04045) value / 12.92 else Math.pow((value + 0.055) / 1.055, 2.4)
    }
}
