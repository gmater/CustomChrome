export const SCREEN_W = 240;
export const SCREEN_H = 135;

export type Fit = "cover" | "contain" | "stretch";

export interface SourceFrame {
  bitmap: ImageBitmap;
  delayCs: number;
}

export interface RenderOptions {
  fit: Fit;
  background: string;
  speed: number;
}

interface DecodedVideoFrame {
  displayWidth: number;
  displayHeight: number;
  duration: number | null;
  close(): void;
}

interface ImageDecoderLike {
  tracks: { ready: Promise<void>; selectedTrack: { frameCount: number } | null };
  decode(opts: { frameIndex: number }): Promise<{ image: DecodedVideoFrame }>;
  close(): void;
}

type ImageDecoderCtor = new (init: { data: ArrayBuffer; type: string }) => ImageDecoderLike;

const DEFAULT_DELAY_CS = 10;

export async function decodeImage(file: File, maxFrames = 255): Promise<SourceFrame[]> {
  const Decoder = (globalThis as { ImageDecoder?: ImageDecoderCtor }).ImageDecoder;
  if (Decoder && file.type) {
    const decoder = new Decoder({ data: await file.arrayBuffer(), type: file.type });
    try {
      await decoder.tracks.ready;
      const count = Math.min(decoder.tracks.selectedTrack?.frameCount ?? 1, maxFrames);
      const frames: SourceFrame[] = [];
      for (let i = 0; i < count; i++) {
        const { image } = await decoder.decode({ frameIndex: i });
        const delayCs = image.duration ? Math.round(image.duration / 10_000) : DEFAULT_DELAY_CS;
        frames.push({
          bitmap: await createImageBitmap(image as unknown as ImageBitmapSource),
          delayCs: delayCs > 0 ? delayCs : DEFAULT_DELAY_CS,
        });
        image.close();
      }
      return frames;
    } finally {
      decoder.close();
    }
  }
  return [{ bitmap: await createImageBitmap(file), delayCs: DEFAULT_DELAY_CS }];
}

export function drawFrame(ctx: CanvasRenderingContext2D, bitmap: ImageBitmap, opts: RenderOptions) {
  ctx.fillStyle = opts.background;
  ctx.fillRect(0, 0, SCREEN_W, SCREEN_H);
  ctx.imageSmoothingQuality = "high";
  if (opts.fit === "stretch") {
    ctx.drawImage(bitmap, 0, 0, SCREEN_W, SCREEN_H);
    return;
  }
  const scale = (opts.fit === "cover" ? Math.max : Math.min)(
    SCREEN_W / bitmap.width,
    SCREEN_H / bitmap.height,
  );
  const w = bitmap.width * scale;
  const h = bitmap.height * scale;
  ctx.drawImage(bitmap, (SCREEN_W - w) / 2, (SCREEN_H - h) / 2, w, h);
}

export function scaledDelays(frames: SourceFrame[], speed: number): number[] {
  return frames.map((f) => Math.min(51, Math.max(1, Math.round(f.delayCs / speed))));
}

export function renderAll(frames: SourceFrame[], opts: RenderOptions): Uint8Array {
  const canvas = new OffscreenCanvas(SCREEN_W, SCREEN_H);
  const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
  const frameBytes = SCREEN_W * SCREEN_H * 4;
  const out = new Uint8Array(frameBytes * frames.length);
  frames.forEach((f, i) => {
    drawFrame(ctx as unknown as CanvasRenderingContext2D, f.bitmap, opts);
    out.set(ctx.getImageData(0, 0, SCREEN_W, SCREEN_H).data, i * frameBytes);
  });
  return out;
}
