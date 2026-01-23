import { useState, useCallback } from 'react'
import Cropper from 'react-easy-crop'
import { X, ZoomIn, ZoomOut, RotateCw } from 'lucide-react'
import { Button } from './ui/button'

interface Point {
  x: number
  y: number
}

interface Area {
  x: number
  y: number
  width: number
  height: number
}

interface AvatarCropDialogProps {
  imageUrl: string
  onCrop: (croppedBlob: Blob) => void
  onCancel: () => void
}

export function AvatarCropDialog({ imageUrl, onCrop, onCancel }: AvatarCropDialogProps) {
  const [crop, setCrop] = useState<Point>({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const [rotation, setRotation] = useState(0)
  const [croppedAreaPixels, setCroppedAreaPixels] = useState<Area | null>(null)
  const [isProcessing, setIsProcessing] = useState(false)

  const onCropComplete = useCallback((_: Area, croppedAreaPixels: Area) => {
    setCroppedAreaPixels(croppedAreaPixels)
  }, [])

  const handleConfirm = async () => {
    if (!croppedAreaPixels || isProcessing) return

    setIsProcessing(true)
    try {
      const croppedBlob = await getCroppedImg(imageUrl, croppedAreaPixels, rotation)
      if (croppedBlob) {
        onCrop(croppedBlob)
      }
    } catch (error) {
      console.error('Failed to crop image:', error)
    } finally {
      setIsProcessing(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onCancel} />

      {/* Dialog */}
      <div className="relative bg-white rounded-xl shadow-xl max-w-lg w-full mx-4 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold text-gray-900">裁剪头像</h2>
          <button
            onClick={onCancel}
            className="p-1 rounded-lg hover:bg-gray-100 transition-colors"
          >
            <X className="h-5 w-5 text-gray-500" />
          </button>
        </div>

        {/* Cropper Area */}
        <div className="relative h-80 bg-gray-900">
          <Cropper
            image={imageUrl}
            crop={crop}
            zoom={zoom}
            rotation={rotation}
            aspect={1}
            cropShape="round"
            showGrid={false}
            onCropChange={setCrop}
            onZoomChange={setZoom}
            onCropComplete={onCropComplete}
          />
        </div>

        {/* Controls */}
        <div className="p-4 space-y-4">
          {/* Zoom Control */}
          <div className="flex items-center gap-3">
            <ZoomOut className="h-4 w-4 text-gray-500" />
            <input
              type="range"
              min={1}
              max={3}
              step={0.1}
              value={zoom}
              onChange={(e) => setZoom(Number(e.target.value))}
              className="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-orange-500"
            />
            <ZoomIn className="h-4 w-4 text-gray-500" />
          </div>

          {/* Rotation Control */}
          <div className="flex items-center gap-3">
            <RotateCw className="h-4 w-4 text-gray-500" />
            <input
              type="range"
              min={0}
              max={360}
              step={1}
              value={rotation}
              onChange={(e) => setRotation(Number(e.target.value))}
              className="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-orange-500"
            />
            <span className="text-sm text-gray-500 w-12 text-right">{rotation}°</span>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 p-4 border-t">
          <Button variant="outline" onClick={onCancel} disabled={isProcessing}>
            取消
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isProcessing}
            className="bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600"
          >
            {isProcessing ? '处理中...' : '确定'}
          </Button>
        </div>
      </div>
    </div>
  )
}

// Utility function to create an image element from URL
function createImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.addEventListener('load', () => resolve(image))
    image.addEventListener('error', (error) => reject(error))
    image.crossOrigin = 'anonymous'
    image.src = url
  })
}

// Calculate rotated bounding box size
function rotateSize(width: number, height: number, rotation: number) {
  const rotRad = (rotation * Math.PI) / 180
  return {
    width: Math.abs(Math.cos(rotRad) * width) + Math.abs(Math.sin(rotRad) * height),
    height: Math.abs(Math.sin(rotRad) * width) + Math.abs(Math.cos(rotRad) * height),
  }
}

// Crop the image and return as Blob
async function getCroppedImg(
  imageSrc: string,
  pixelCrop: Area,
  rotation: number = 0
): Promise<Blob | null> {
  const image = await createImage(imageSrc)
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')

  if (!ctx) return null

  const rotRad = (rotation * Math.PI) / 180

  // Calculate bounding box of the rotated image
  const { width: bBoxWidth, height: bBoxHeight } = rotateSize(
    image.width,
    image.height,
    rotation
  )

  // Set canvas size to match the bounding box
  canvas.width = bBoxWidth
  canvas.height = bBoxHeight

  // Translate and rotate
  ctx.translate(bBoxWidth / 2, bBoxHeight / 2)
  ctx.rotate(rotRad)
  ctx.translate(-image.width / 2, -image.height / 2)

  // Draw the rotated image
  ctx.drawImage(image, 0, 0)

  // Create a new canvas for the cropped result
  const croppedCanvas = document.createElement('canvas')
  const croppedCtx = croppedCanvas.getContext('2d')

  if (!croppedCtx) return null

  // Set the size of the cropped canvas
  croppedCanvas.width = pixelCrop.width
  croppedCanvas.height = pixelCrop.height

  // Draw the cropped image
  croppedCtx.drawImage(
    canvas,
    pixelCrop.x,
    pixelCrop.y,
    pixelCrop.width,
    pixelCrop.height,
    0,
    0,
    pixelCrop.width,
    pixelCrop.height
  )

  // Return as Blob (JPEG format for smaller size)
  return new Promise((resolve) => {
    croppedCanvas.toBlob(
      (blob) => resolve(blob),
      'image/jpeg',
      0.95
    )
  })
}
