package com.familyroom.app

import java.util.Calendar
import java.util.TimeZone

internal fun datePickerMillisForCapture(
    capturedAtSeconds: Long,
    timeZone: TimeZone = TimeZone.getDefault(),
): Long {
    val local = Calendar.getInstance(timeZone).apply {
        timeInMillis = capturedAtSeconds * 1_000L
    }
    return Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
        clear()
        set(local.get(Calendar.YEAR), local.get(Calendar.MONTH), local.get(Calendar.DAY_OF_MONTH))
    }.timeInMillis
}

internal fun correctedCaptureTimestamp(
    originalSeconds: Long,
    selectedDateMillis: Long,
    timeZone: TimeZone = TimeZone.getDefault(),
): Long {
    val original = Calendar.getInstance(timeZone).apply {
        timeInMillis = originalSeconds * 1_000L
    }
    val selectedDay = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
        timeInMillis = selectedDateMillis
    }
    original.set(Calendar.YEAR, selectedDay.get(Calendar.YEAR))
    original.set(Calendar.MONTH, selectedDay.get(Calendar.MONTH))
    original.set(Calendar.DAY_OF_MONTH, selectedDay.get(Calendar.DAY_OF_MONTH))
    return original.timeInMillis / 1_000L
}
