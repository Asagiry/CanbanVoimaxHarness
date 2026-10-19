/**
 * Client-Side Image Compression Utility
 * 
 * Compresses oversized screenshots and images on an offscreen HTML5 <canvas>
 * before transmitting over Tauri IPC / LLM APIs.
 * 
 * - Max dimension constrained to 1568px (Anthropic Claude & OpenAI Vision optimal standard).
 * - JPEG quality 0.85 downscales 10MB screenshots to ~150-200KB instantly.
 * - Intelligently checks PNG transparency, preserving transparent PNGs when compact,
 *   and compositing over crisp white background when converting to JPEG.
 */

export async function compressImageFile(
  file: File,
  maxDimension = 1568,
  quality = 0.85
): Promise<string> {
  // SVG vectors do not require raster canvas compression; preserve crispness
  if (file.type === 'image/svg+xml' || file.name.toLowerCase().endsWith('.svg')) {
    return readFileAsDataUrl(file);
  }

  // If browser environment lacks canvas or window, fallback
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return readFileAsDataUrl(file);
  }

  return new Promise<string>((resolve, reject) => {
    let objectUrl: string | null = null;
    try {
      objectUrl = URL.createObjectURL(file);
    } catch {
      // Fallback if createObjectURL is unavailable
      readFileAsDataUrl(file).then(resolve).catch(reject);
      return;
    }

    const img = new Image();

    img.onload = () => {
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrl = null;
      }

      try {
        let width = img.naturalWidth || img.width;
        let height = img.naturalHeight || img.height;

        if (!width || !height) {
          readFileAsDataUrl(file).then(resolve).catch(reject);
          return;
        }

        // Downscale proportionally if max dimension is exceeded
        const maxSide = Math.max(width, height);
        if (maxSide > maxDimension) {
          const scale = maxDimension / maxSide;
          width = Math.round(width * scale);
          height = Math.round(height * scale);
        }

        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });

        if (!ctx) {
          readFileAsDataUrl(file).then(resolve).catch(reject);
          return;
        }

        // Draw image onto offscreen canvas
        ctx.drawImage(img, 0, 0, width, height);

        // Check if PNG has transparency
        const isPng = file.type === 'image/png' || file.name.toLowerCase().endsWith('.png');
        let hasAlpha = false;

        if (isPng) {
          try {
            const imgData = ctx.getImageData(0, 0, width, height).data;
            // Sample pixels for speed (step of ~40,000 checks total)
            const step = Math.max(1, Math.floor(imgData.length / 40000));
            for (let i = 3; i < imgData.length; i += 4 * step) {
              if (imgData[i] < 250) {
                hasAlpha = true;
                break;
              }
            }
          } catch {
            // In case of security/canvas read restriction, treat as standard
          }

          if (hasAlpha) {
            const pngDataUrl = canvas.toDataURL('image/png');
            // If PNG with transparency is reasonably sized (< 1MB ~ 1.33M base64 chars),
            // preserve PNG format so transparent backgrounds don't get lost
            if (pngDataUrl.length < 1_350_000) {
              resolve(pngDataUrl);
              return;
            }
          }
        }

        // For JPEG output, composite over white background so transparent areas do not become black
        const jpegCanvas = document.createElement('canvas');
        jpegCanvas.width = width;
        jpegCanvas.height = height;
        const jpegCtx = jpegCanvas.getContext('2d');

        if (jpegCtx) {
          jpegCtx.fillStyle = '#ffffff';
          jpegCtx.fillRect(0, 0, width, height);
          jpegCtx.drawImage(canvas, 0, 0);
          resolve(jpegCanvas.toDataURL('image/jpeg', quality));
        } else {
          resolve(canvas.toDataURL('image/jpeg', quality));
        }
      } catch (err) {
        console.warn('[compressImageFile] Canvas compression error, falling back to original:', err);
        readFileAsDataUrl(file).then(resolve).catch(reject);
      }
    };

    img.onerror = (err) => {
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrl = null;
      }
      console.warn('[compressImageFile] Image load error, falling back to original:', err);
      readFileAsDataUrl(file).then(resolve).catch(reject);
    };

    img.src = objectUrl;
  });
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}
