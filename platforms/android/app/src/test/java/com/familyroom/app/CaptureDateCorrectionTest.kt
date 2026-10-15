package com.familyroom.app

import java.util.Calendar
import java.util.TimeZone
import org.junit.Assert.assertEquals
import org.junit.Test

class CaptureDateCorrectionTest {
    @Test
    fun correctionChangesCalendarDayButPreservesLocalTimeAndSpacing() {
        val zone = TimeZone.getTimeZone("GMT-05:00")
        val original = Calendar.getInstance(zone).apply {
            clear()
            set(2020, Calendar.FEBRUARY, 3, 14, 27, 31)
        }.timeInMillis / 1_000L
        val selectedDate = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(2024, Calendar.NOVEMBER, 5, 0, 0, 0)
        }.timeInMillis
        val pickerMillis = datePickerMillisForCapture(original, zone)
        val pickerDay = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            timeInMillis = pickerMillis
        }
        assertEquals(Calendar.FEBRUARY, pickerDay.get(Calendar.MONTH))
        assertEquals(3, pickerDay.get(Calendar.DAY_OF_MONTH))

        val corrected = correctedCaptureTimestamp(original, selectedDate, zone)
        val next = correctedCaptureTimestamp(original + 3_600, selectedDate, zone)
        val local = Calendar.getInstance(zone).apply { timeInMillis = corrected * 1_000L }

        assertEquals(2024, local.get(Calendar.YEAR))
        assertEquals(Calendar.NOVEMBER, local.get(Calendar.MONTH))
        assertEquals(5, local.get(Calendar.DAY_OF_MONTH))
        assertEquals(14, local.get(Calendar.HOUR_OF_DAY))
        assertEquals(27, local.get(Calendar.MINUTE))
        assertEquals(31, local.get(Calendar.SECOND))
        assertEquals(3_600, next - corrected)
    }
}
