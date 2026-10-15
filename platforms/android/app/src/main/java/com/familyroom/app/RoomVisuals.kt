package com.familyroom.app

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import android.provider.Settings
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.zIndex
import androidx.compose.ui.platform.testTag
import kotlinx.coroutines.delay
import androidx.compose.foundation.LocalIndication
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONObject

val RoomDisplay = FontFamily(Font(R.font.instrument_serif))

@Composable fun roomGalleryBackground(): Brush = Brush.verticalGradient(listOf(
    MaterialTheme.colorScheme.background, MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)))

/** A featured memory and separate browsable strip, keeping the Room's own layout. */
@Composable fun RoomHighlights(assets: List<JSONObject>,
    thumbnail: @Composable (JSONObject, Modifier) -> Unit,
    onOpen: (JSONObject) -> Unit,
    title: String = "In the spotlight",
    subtitle: String? = null) {
    if (assets.isEmpty()) return
    val context = LocalContext.current
    val reduceMotion = remember(context) {
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    var entered by remember(assets.first().optString("id"), reduceMotion) { mutableStateOf(reduceMotion) }
    LaunchedEffect(assets.first().optString("id"), reduceMotion) {
        if (reduceMotion) entered = true else {
            delay(50)
            entered = true
        }
    }
    val progress by animateFloatAsState(
        targetValue = if (entered || reduceMotion) 1f else 0f,
        animationSpec = spring(dampingRatio = 0.78f, stiffness = 280f),
        label = "featured photo stack entrance",
    )
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text(title, fontFamily = RoomDisplay, fontSize = 30.sp, color = MaterialTheme.colorScheme.onSurface)
        if (subtitle != null) Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        BoxWithConstraints(Modifier.fillMaxWidth().height(300.dp).testTag("home.featured.stack")) {
            val photoWidth = maxWidth - 20.dp
            val photoHeight = maxHeight - 12.dp
            if (assets.size > 2) {
                RoomPhotoFrame(
                    Modifier.size(photoWidth, photoHeight)
                        .align(Alignment.Center)
                        .graphicsLayer {
                            val settle = progress
                            scaleX = 0.84f
                            scaleY = 0.84f
                            rotationZ = 3.5f + (1f - settle) * 2f
                            rotationY = 7f * (1f - settle)
                            translationX = constraints.maxWidth * 0.12f * settle
                            translationY = -10.dp.toPx() * settle
                            alpha = settle
                            cameraDistance = 18f * density
                            shadowElevation = 8.dp.toPx()
                        }
                        .zIndex(0f)
                        .clearAndSetSemantics {},
                ) {
                    thumbnail(assets[2], Modifier.fillMaxSize())
                }
            }
            if (assets.size > 1) {
                RoomPhotoFrame(
                    Modifier.size(photoWidth, photoHeight)
                        .align(Alignment.Center)
                        .graphicsLayer {
                            val settle = progress
                            scaleX = 0.92f
                            scaleY = 0.92f
                            rotationZ = -2.5f - (1f - settle) * 2f
                            rotationY = -5f * (1f - settle)
                            translationX = -constraints.maxWidth * 0.08f * settle
                            translationY = 7.dp.toPx() * settle
                            alpha = settle
                            cameraDistance = 18f * density
                            shadowElevation = 8.dp.toPx()
                        }
                        .zIndex(1f)
                        .clearAndSetSemantics {},
                ) {
                    thumbnail(assets[1], Modifier.fillMaxSize())
                }
            }
            PressableMemory(
                Modifier.fillMaxSize()
                    .graphicsLayer {
                        val settle = progress
                        scaleX = 0.96f + 0.04f * settle
                        scaleY = 0.96f + 0.04f * settle
                        rotationX = 4f * (1f - settle)
                        rotationY = -5f * (1f - settle)
                        translationY = 20.dp.toPx() * (1f - settle)
                        alpha = settle
                        cameraDistance = 18f * density
                    }
                    .zIndex(2f),
                onClick = { onOpen(assets.first()) },
            ) {
                thumbnail(assets.first(), Modifier.fillMaxSize())
            }
        }
        if (assets.size > 1) Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 3.dp), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            for (asset in assets.drop(1).take(4)) PressableMemory(Modifier.size(160.dp, 108.dp), onClick = { onOpen(asset) }) {
                thumbnail(asset, Modifier.fillMaxSize())
            }
        }
    }
}

@Composable
private fun PressableMemory(modifier: Modifier, onClick: () -> Unit, content: @Composable BoxScope.() -> Unit) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val context = LocalContext.current
    val reduced = remember(context) {
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    val scale by animateFloatAsState(if (pressed && !reduced) 0.975f else 1f,
        animationSpec = spring(dampingRatio = 0.72f), label = "memory press")
    Box(modifier.graphicsLayer { scaleX = scale; scaleY = scale }
        .clickable(interactionSource = source, indication = LocalIndication.current, onClick = onClick),
        content = content)
}

/** Always black and white around a full-color image, including in dark mode. */
@Composable fun RoomPhotoFrame(modifier: Modifier, content: @Composable BoxScope.() -> Unit) {
    Box(modifier.background(Color.White, RoundedCornerShape(8.dp))
        .border(1.dp, Color.Black, RoundedCornerShape(8.dp)).padding(5.dp)) {
        Box(Modifier.fillMaxSize().clip(RoundedCornerShape(3.dp)).border(0.7.dp, Color.Black, RoundedCornerShape(3.dp)), content = content)
    }
}
