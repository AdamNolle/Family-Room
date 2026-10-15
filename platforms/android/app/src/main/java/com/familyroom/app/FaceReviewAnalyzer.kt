package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import androidx.exifinterface.media.ExifInterface
import com.google.android.gms.tasks.Tasks
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.face.FaceDetector
import com.google.mlkit.vision.face.FaceDetectorOptions
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.concurrent.TimeUnit
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min

private const val MIN_CROP_PIXELS = 24

internal data class FaceCrop(
    val id: String,
    val assetId: String,
    val jpeg: ByteArray,
)

internal data class FaceCropBounds(
    val left: Int,
    val top: Int,
    val right: Int,
    val bottom: Int,
) {
    val width: Int get() = right - left
    val height: Int get() = bottom - top
}

internal fun faceCropBounds(
    left: Int,
    top: Int,
    right: Int,
    bottom: Int,
    imageWidth: Int,
    imageHeight: Int,
    marginRatio: Float = 0.16f,
): FaceCropBounds? {
    if (imageWidth <= 0 || imageHeight <= 0 || marginRatio < 0f) return null
    val boundedLeft = left.coerceIn(0, imageWidth)
    val boundedTop = top.coerceIn(0, imageHeight)
    val boundedRight = right.coerceIn(0, imageWidth)
    val boundedBottom = bottom.coerceIn(0, imageHeight)
    if (boundedRight <= boundedLeft || boundedBottom <= boundedTop) return null

    val horizontalMargin = ceil((boundedRight - boundedLeft) * marginRatio).toInt()
    val verticalMargin = ceil((boundedBottom - boundedTop) * marginRatio).toInt()
    val result = FaceCropBounds(
        left = max(0, boundedLeft - horizontalMargin),
        top = max(0, boundedTop - verticalMargin),
        right = min(imageWidth, boundedRight + horizontalMargin),
        bottom = min(imageHeight, boundedBottom + verticalMargin),
    )
    return result.takeIf { it.width >= MIN_CROP_PIXELS && it.height >= MIN_CROP_PIXELS }
}

internal object FaceReviewAnalyzer {
    const val MAX_CROPS_PER_PHOTO = 24
    const val MAX_REVIEW_CROPS = 160
    private const val MAX_IMAGE_DIMENSION = 2_048
    private const val MAX_CROP_DIMENSION = 420

    fun createDetector(): FaceDetector = com.google.mlkit.vision.face.FaceDetection.getClient(
        FaceDetectorOptions.Builder()
            .setPerformanceMode(FaceDetectorOptions.PERFORMANCE_MODE_ACCURATE)
            .setLandmarkMode(FaceDetectorOptions.LANDMARK_MODE_NONE)
            .setClassificationMode(FaceDetectorOptions.CLASSIFICATION_MODE_NONE)
            .setMinFaceSize(0.035f)
            .build(),
    )

    suspend fun review(file: File, assetId: String, detector: FaceDetector): List<FaceCrop> =
        withContext(Dispatchers.IO) {
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeFile(file.absolutePath, bounds)
            if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return@withContext emptyList()

            var sampleSize = 1
            while (sampleSize < (1 shl 30) &&
                max(bounds.outWidth / sampleSize, bounds.outHeight / sampleSize) > MAX_IMAGE_DIMENSION
            ) {
                sampleSize *= 2
            }
            val options = BitmapFactory.Options().apply {
                inSampleSize = sampleSize
                inPreferredConfig = Bitmap.Config.ARGB_8888
            }
            val decoded = BitmapFactory.decodeFile(file.absolutePath, options) ?: return@withContext emptyList()
            val oriented = orient(decoded, file)
            try {
                val input = InputImage.fromBitmap(oriented, 0)
                val detected = Tasks.await(detector.process(input), 30, TimeUnit.SECONDS)
                detected.asSequence().take(MAX_CROPS_PER_PHOTO).mapIndexedNotNull { index, face ->
                    val faceBounds = face.boundingBox
                    val cropBounds = faceCropBounds(
                        left = faceBounds.left,
                        top = faceBounds.top,
                        right = faceBounds.right,
                        bottom = faceBounds.bottom,
                        imageWidth = oriented.width,
                        imageHeight = oriented.height,
                    ) ?: return@mapIndexedNotNull null

                    val crop = Bitmap.createBitmap(
                        oriented,
                        cropBounds.left,
                        cropBounds.top,
                        cropBounds.width,
                        cropBounds.height,
                    )
            var preview: Bitmap? = null
            try {
                val renderedPreview = scalePreview(crop)
                preview = renderedPreview
                val bytes = ByteArrayOutputStream().use { output ->
                    check(renderedPreview.compress(Bitmap.CompressFormat.JPEG, 82, output)) {
                        "Could not prepare a temporary face crop."
                    }
                    output.toByteArray()
                }
                FaceCrop(id = "$assetId:$index", assetId = assetId, jpeg = bytes)
            } finally {
                preview?.takeIf { it !== crop }?.recycle()
                crop.recycle()
            }
                }.toList()
            } finally {
                oriented.recycle()
            }
        }

    private fun orient(bitmap: Bitmap, file: File): Bitmap {
        val exif = runCatching { ExifInterface(file) }.getOrNull() ?: return bitmap
        val rotation = exif.rotationDegrees
        if (rotation == 0 && !exif.isFlipped) return bitmap

        val transform = Matrix()
        if (exif.isFlipped) transform.postScale(-1f, 1f)
        if (rotation != 0) transform.postRotate(rotation.toFloat())
        return runCatching {
            Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, transform, true)
        }.getOrElse { bitmap }.also { oriented ->
            if (oriented !== bitmap) bitmap.recycle()
        }
    }

    private fun scalePreview(crop: Bitmap): Bitmap {
        val largestSide = max(crop.width, crop.height)
        if (largestSide <= MAX_CROP_DIMENSION) return crop
        val ratio = MAX_CROP_DIMENSION.toFloat() / largestSide
        return Bitmap.createScaledBitmap(
            crop,
            max(1, (crop.width * ratio).toInt()),
            max(1, (crop.height * ratio).toInt()),
            true,
        )
    }
}
