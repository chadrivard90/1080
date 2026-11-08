// Image prep for the art folder (resize / crop / background removal), used by tools/prep-art.ps1.
// Compiled on the fly by PowerShell (Add-Type), so no extra software is needed on Windows.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class ArtTool
{
    static ImageCodecInfo Jpeg() {
        foreach (var c in ImageCodecInfo.GetImageEncoders()) if (c.MimeType == "image/jpeg") return c;
        return null;
    }

    public static void SaveJpg(Bitmap b, string path, long quality) {
        var p = new EncoderParameters(1);
        p.Param[0] = new EncoderParameter(Encoder.Quality, quality);
        b.Save(path, Jpeg(), p);
    }

    static Bitmap Draw(Bitmap src, int w, int h, RectangleF from) {
        var dst = new Bitmap(w, h, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(dst)) {
            g.Clear(Color.Transparent);
            g.InterpolationMode = InterpolationMode.HighQualityBicubic;
            g.PixelOffsetMode = PixelOffsetMode.HighQuality;
            g.CompositingQuality = CompositingQuality.HighQuality;
            using (var ia = new ImageAttributes()) {
                ia.SetWrapMode(WrapMode.TileFlipXY);
                g.DrawImage(src, new Rectangle(0, 0, w, h), from.X, from.Y, from.Width, from.Height, GraphicsUnit.Pixel, ia);
            }
        }
        return dst;
    }

    /// Scale to cover w x h and centre-crop.
    public static Bitmap Cover(Bitmap src, int w, int h) {
        float s = Math.Max((float)w / src.Width, (float)h / src.Height);
        float cw = w / s, ch = h / s;
        return Draw(src, w, h, new RectangleF((src.Width - cw) / 2, (src.Height - ch) / 2, cw, ch));
    }

    static byte[] Bytes(Bitmap b, out BitmapData d) {
        d = b.LockBits(new Rectangle(0, 0, b.Width, b.Height), ImageLockMode.ReadWrite, PixelFormat.Format32bppArgb);
        var buf = new byte[d.Stride * b.Height];
        Marshal.Copy(d.Scan0, buf, 0, buf.Length);
        return buf;
    }

    static void Put(Bitmap b, BitmapData d, byte[] buf) {
        Marshal.Copy(buf, 0, d.Scan0, buf.Length);
        b.UnlockBits(d);
    }

    public static Bitmap To32(Bitmap src) {
        var b = new Bitmap(src.Width, src.Height, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(b)) g.DrawImage(src, 0, 0, src.Width, src.Height);
        return b;
    }

    /// Bounding box of pixels brighter than `lum` (for finding a badge on black).
    public static Rectangle BrightBox(Bitmap src, int lum) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        int minX = b.Width, minY = b.Height, maxX = -1, maxY = -1;
        for (int y = 0; y < b.Height; y++)
            for (int x = 0; x < b.Width; x++) {
                int i = y * d.Stride + x * 4;
                int m = Math.Max(buf[i], Math.Max(buf[i + 1], buf[i + 2]));
                if (m > lum) { if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y; }
            }
        b.UnlockBits(d); b.Dispose();
        if (maxX < 0) return new Rectangle(0, 0, src.Width, src.Height);
        return Rectangle.FromLTRB(minX, minY, maxX + 1, maxY + 1);
    }

    /// Square crop centred on `box` (with margin), scaled to size x size.
    public static Bitmap SquareAround(Bitmap src, Rectangle box, float margin, int size) {
        float side = Math.Max(box.Width, box.Height) * (1 + margin);
        float cx = box.X + box.Width / 2f, cy = box.Y + box.Height / 2f;
        return Draw(src, size, size, new RectangleF(cx - side / 2, cy - side / 2, side, side));
    }

    /// Make the near-black background transparent: flood fill from the edges through pixels whose
    /// brightest channel is <= `hard`, then feather pixels next to the removed area up to `soft`.
    public static Bitmap RemoveBlack(Bitmap src, int hard, int soft) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        int w = b.Width, h = b.Height;
        var bg = new bool[w * h];
        var q = new Queue<int>();
        Func<int, int> maxc = (p) => { int i = (p / w) * d.Stride + (p % w) * 4; return Math.Max(buf[i], Math.Max(buf[i + 1], buf[i + 2])); };
        for (int x = 0; x < w; x++) { q.Enqueue(x); q.Enqueue((h - 1) * w + x); }
        for (int y = 0; y < h; y++) { q.Enqueue(y * w); q.Enqueue(y * w + w - 1); }
        while (q.Count > 0) {
            int p = q.Dequeue();
            if (bg[p] || maxc(p) > hard) continue;
            bg[p] = true;
            int x = p % w, y = p / w;
            if (x > 0) q.Enqueue(p - 1);
            if (x < w - 1) q.Enqueue(p + 1);
            if (y > 0) q.Enqueue(p - w);
            if (y < h - 1) q.Enqueue(p + w);
        }
        for (int p = 0; p < w * h; p++) {
            int i = (p / w) * d.Stride + (p % w) * 4;
            if (bg[p]) { buf[i + 3] = 0; continue; }
            int x = p % w, y = p / w;
            bool edge = (x > 0 && bg[p - 1]) || (x < w - 1 && bg[p + 1]) || (y > 0 && bg[p - w]) || (y < h - 1 && bg[p + w]);
            if (edge) {
                int m = maxc(p);
                if (m < soft) buf[i + 3] = (byte)Math.Max(0, Math.Min(255, (m - hard) * 255 / Math.Max(1, soft - hard)));
            }
        }
        Put(b, d, buf);
        return b;
    }

    /// For subjects with a glow on black: everything connected to the edge through pixels darker
    /// than `core` becomes background, and background pixels are "un-premultiplied" from black
    /// (alpha = brightness), so glows fade smoothly to transparent instead of leaving a halo.
    public static Bitmap RemoveBlackGlow(Bitmap src, int core) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        int w = b.Width, h = b.Height;
        var bg = new bool[w * h];
        var q = new Queue<int>();
        Func<int, int> maxc = (p) => { int i = (p / w) * d.Stride + (p % w) * 4; return Math.Max(buf[i], Math.Max(buf[i + 1], buf[i + 2])); };
        for (int x = 0; x < w; x++) { q.Enqueue(x); q.Enqueue((h - 1) * w + x); }
        for (int y = 0; y < h; y++) { q.Enqueue(y * w); q.Enqueue(y * w + w - 1); }
        while (q.Count > 0) {
            int p = q.Dequeue();
            if (bg[p] || maxc(p) > core) continue;
            bg[p] = true;
            int x = p % w, y = p / w;
            if (x > 0) q.Enqueue(p - 1);
            if (x < w - 1) q.Enqueue(p + 1);
            if (y > 0) q.Enqueue(p - w);
            if (y < h - 1) q.Enqueue(p + w);
        }
        for (int p = 0; p < w * h; p++) {
            if (!bg[p]) continue;
            int i = (p / w) * d.Stride + (p % w) * 4;
            int m = Math.Max(buf[i], Math.Max(buf[i + 1], buf[i + 2]));
            int a = m < 12 ? 0 : m;
            buf[i + 3] = (byte)a;
            if (a > 0) {
                buf[i] = (byte)Math.Min(255, buf[i] * 255 / a);
                buf[i + 1] = (byte)Math.Min(255, buf[i + 1] * 255 / a);
                buf[i + 2] = (byte)Math.Min(255, buf[i + 2] * 255 / a);
            }
        }
        Put(b, d, buf);
        return b;
    }

    /// Key out a flat background colour (e.g. pure magenta): pixels within `tol` of it become
    /// transparent, pixels up to 2 x `tol` are feathered. Used where the subject itself can be
    /// black or white (flags, tyres).
    public static Bitmap RemoveColor(Bitmap src, int r, int g, int bl, int tol) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        for (int y = 0; y < b.Height; y++)
            for (int x = 0; x < b.Width; x++) {
                int i = y * d.Stride + x * 4;
                int dist = Math.Max(Math.Abs(buf[i + 2] - r), Math.Max(Math.Abs(buf[i + 1] - g), Math.Abs(buf[i] - bl)));
                if (dist <= tol) buf[i + 3] = 0;
                else if (dist < tol * 2) buf[i + 3] = (byte)Math.Min(buf[i + 3], (dist - tol) * 255 / tol);
            }
        Put(b, d, buf);
        return b;
    }

    /// Crop away solid bands of one colour along any edge (Gemini sometimes adds a beige strip).
    /// A row / column counts as band when 90% of its pixels are within `tol` of the colour.
    public static Bitmap TrimBands(Bitmap src, int r, int g, int bl, int tol) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        int w = b.Width, h = b.Height;
        Func<int, int, bool> near = (x, y) => {
            int i = y * d.Stride + x * 4;
            return Math.Abs(buf[i + 2] - r) <= tol && Math.Abs(buf[i + 1] - g) <= tol && Math.Abs(buf[i] - bl) <= tol;
        };
        Func<int, bool> rowBand = (y) => { int c = 0; for (int x = 0; x < w; x++) if (near(x, y)) c++; return c > w * 0.9; };
        Func<int, bool> colBand = (x) => { int c = 0; for (int y = 0; y < h; y++) if (near(x, y)) c++; return c > h * 0.9; };
        int top = 0, bottom = h - 1, left = 0, right = w - 1;
        while (top < bottom && rowBand(top)) top++;
        while (bottom > top && rowBand(bottom)) bottom--;
        while (left < right && colBand(left)) left++;
        while (right > left && colBand(right)) right--;
        b.UnlockBits(d);
        if (top == 0 && left == 0 && bottom == h - 1 && right == w - 1) return b;
        // Shave a few extra pixels to lose the soft edge of the band.
        int m = 4;
        var rect = Rectangle.FromLTRB(Math.Min(left + (left > 0 ? m : 0), w - 1), Math.Min(top + (top > 0 ? m : 0), h - 1),
                                      Math.Max(right - (right < w - 1 ? m : 0), 1) + 1, Math.Max(bottom - (bottom < h - 1 ? m : 0), 1) + 1);
        var cut = b.Clone(rect, PixelFormat.Format32bppArgb);
        b.Dispose();
        return cut;
    }

    /// Mask for badge-style icons with dark insides: on each row, everything between the first and
    /// last pixel brighter than `lum` (the frame) stays opaque; everything outside is transparent.
    public static Bitmap RowSpanMask(Bitmap src, int lum) {
        var b = To32(src);
        BitmapData d; var buf = Bytes(b, out d);
        int w = b.Width, h = b.Height;
        for (int y = 0; y < h; y++) {
            int first = -1, last = -1;
            for (int x = 0; x < w; x++) {
                int i = y * d.Stride + x * 4;
                if (Math.Max(buf[i], Math.Max(buf[i + 1], buf[i + 2])) > lum) { if (first < 0) first = x; last = x; }
            }
            for (int x = 0; x < w; x++) {
                int i = y * d.Stride + x * 4;
                if (first < 0 || x < first || x > last) buf[i + 3] = 0;
            }
        }
        Put(b, d, buf);
        return b;
    }

    /// Bounding box of pixels with alpha > 16.
    public static Rectangle OpaqueBox(Bitmap b) {
        BitmapData d; var buf = Bytes(b, out d);
        int minX = b.Width, minY = b.Height, maxX = -1, maxY = -1;
        for (int y = 0; y < b.Height; y++)
            for (int x = 0; x < b.Width; x++)
                if (buf[y * d.Stride + x * 4 + 3] > 16) { if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y; }
        b.UnlockBits(d);
        if (maxX < 0) return new Rectangle(0, 0, b.Width, b.Height);
        return Rectangle.FromLTRB(minX, minY, maxX + 1, maxY + 1);
    }

    /// Fit `box` of `src` into a w x h transparent canvas (keeping aspect, centred, with margin).
    public static Bitmap FitInto(Bitmap src, Rectangle box, int w, int h, float margin) {
        float s = Math.Min(w * (1 - margin) / box.Width, h * (1 - margin) / box.Height);
        var dst = new Bitmap(w, h, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(dst)) {
            g.Clear(Color.Transparent);
            g.InterpolationMode = InterpolationMode.HighQualityBicubic;
            g.PixelOffsetMode = PixelOffsetMode.HighQuality;
            float dw = box.Width * s, dh = box.Height * s;
            g.DrawImage(src, new RectangleF((w - dw) / 2, (h - dh) / 2, dw, dh), box, GraphicsUnit.Pixel);
        }
        return dst;
    }

    /// Row (y) where the opaque area is widest inside `box`: the needle's pivot hub.
    public static int WidestRow(Bitmap b, Rectangle box) {
        BitmapData d; var buf = Bytes(b, out d);
        int bestY = box.Y, bestW = 0;
        for (int y = box.Top; y < box.Bottom; y++) {
            int c = 0;
            for (int x = box.Left; x < box.Right; x++) if (buf[y * d.Stride + x * 4 + 3] > 128) c++;
            if (c > bestW) { bestW = c; bestY = y; }
        }
        // Centre of the band of rows that are nearly as wide (the hub circle).
        int top = bestY, bot = bestY;
        Func<int, int> width = (yy) => { int c = 0; for (int x = box.Left; x < box.Right; x++) if (buf[yy * d.Stride + x * 4 + 3] > 128) c++; return c; };
        while (top > box.Top && width(top - 1) >= bestW * 0.85) top--;
        while (bot < box.Bottom - 1 && width(bot + 1) >= bestW * 0.85) bot++;
        b.UnlockBits(d);
        return (top + bot) / 2;
    }

    /// Place a needle so its pivot (px, py) sits at the centre of a size x size canvas and its tip
    /// (at tipY) reaches `reach` (fraction of size) above the centre.
    public static Bitmap CentreNeedle(Bitmap src, int px, int py, int tipY, int size, float reach) {
        float s = (size * reach) / Math.Max(1, py - tipY);
        var dst = new Bitmap(size, size, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(dst)) {
            g.Clear(Color.Transparent);
            g.InterpolationMode = InterpolationMode.HighQualityBicubic;
            g.PixelOffsetMode = PixelOffsetMode.HighQuality;
            g.DrawImage(src, new RectangleF(size / 2f - px * s, size / 2f - py * s, src.Width * s, src.Height * s));
        }
        return dst;
    }
}
