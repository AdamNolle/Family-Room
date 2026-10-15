package com.familyroom.app

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

internal fun roomColorScheme(dark: Boolean): ColorScheme =
    if (dark) {
        darkColorScheme(
            primary = Color(0xFFE8B28D),
            onPrimary = Color(0xFF2C2926),
            primaryContainer = Color(0xFF583A2B),
            onPrimaryContainer = Color(0xFFFFE3D2),
            secondary = Color(0xFFB7CEDB),
            onSecondary = Color(0xFF263640),
            secondaryContainer = Color(0xFF303D45),
            onSecondaryContainer = Color(0xFFF6F2EC),
            tertiary = Color(0xFFE8C1A4),
            onTertiary = Color(0xFF392518),
            tertiaryContainer = Color(0xFF4D3526),
            onTertiaryContainer = Color(0xFFFFDFC6),
            outlineVariant = Color(0xFF51473F),
            outline = Color(0xFFAB9786),
            background = Color(0xFF1D1C1B),
            onBackground = Color(0xFFF6F2EC),
            surface = Color(0xFF322D28),
            onSurface = Color(0xFFF6F2EC),
            surfaceVariant = Color(0xFF282521),
            onSurfaceVariant = Color(0xFFC9BFB5),
            surfaceDim = Color(0xFF181716),
            surfaceBright = Color(0xFF453D36),
            surfaceContainerLowest = Color(0xFF171615),
            surfaceContainerLow = Color(0xFF24211E),
            surfaceContainer = Color(0xFF2B2723),
            surfaceContainerHigh = Color(0xFF322D28),
            surfaceContainerHighest = Color(0xFF39332D),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF8A4D32),
            onPrimary = Color.White,
            primaryContainer = Color(0xFFF3DFD2),
            onPrimaryContainer = Color(0xFF462719),
            secondary = Color(0xFF536B79),
            onSecondary = Color.White,
            secondaryContainer = Color(0xFFE2EBF1),
            onSecondaryContainer = Color(0xFF3C515F),
            tertiary = Color(0xFF9A633C),
            onTertiary = Color.White,
            tertiaryContainer = Color(0xFFF3E4D7),
            onTertiaryContainer = Color(0xFF49301F),
            outlineVariant = Color(0xFFDFD7CE),
            outline = Color(0xFF8A786A),
            background = Color(0xFFF7F5F1),
            onBackground = Color(0xFF2C2926),
            surface = Color(0xFFFFFDFC),
            onSurface = Color(0xFF2C2926),
            surfaceVariant = Color(0xFFF0EAE3),
            onSurfaceVariant = Color(0xFF70665F),
            surfaceDim = Color(0xFFE8E1D9),
            surfaceBright = Color(0xFFFFFDFC),
            surfaceContainerLowest = Color.White,
            surfaceContainerLow = Color(0xFFFBF8F4),
            surfaceContainer = Color(0xFFF4EFE9),
            surfaceContainerHigh = Color(0xFFEEE8E0),
            surfaceContainerHighest = Color(0xFFE8E0D7),
        )
    }

internal data class RoomTextContrastPair(
    val label: String,
    val foreground: Color,
    val background: Color,
)

internal fun roomTextContrastPairs(scheme: ColorScheme): List<RoomTextContrastPair> =
    listOf(
        RoomTextContrastPair("body text", scheme.onBackground, scheme.background),
        RoomTextContrastPair("surface text", scheme.onSurface, scheme.surface),
        RoomTextContrastPair("secondary text", scheme.onSurfaceVariant, scheme.surfaceVariant),
        RoomTextContrastPair("primary action", scheme.onPrimary, scheme.primary),
        RoomTextContrastPair("blue secondary surface", scheme.onSecondaryContainer, scheme.secondaryContainer),
        RoomTextContrastPair("soft blue accent", scheme.onSecondary, scheme.secondary),
        RoomTextContrastPair("warm primary surface", scheme.onPrimaryContainer, scheme.primaryContainer),
        RoomTextContrastPair("warm accent surface", scheme.onTertiaryContainer, scheme.tertiaryContainer),
        RoomTextContrastPair("card surface", scheme.onSurface, scheme.surfaceContainerLow),
    )
