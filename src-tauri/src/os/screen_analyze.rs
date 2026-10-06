//! Screenshot du moniteur (hors HWND MIND) + OCR Windows.Media.Ocr → brouillon CRM.

use serde::Serialize;
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;

#[cfg(windows)]
pub use super::screen_crm::parse_crm_from_ocr;
pub use super::screen_crm::ScreenCrmDraft;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeScreenResult {
    #[serde(flatten)]
    pub draft: ScreenCrmDraft,
    pub found_text: bool,
}

#[tauri::command]
pub async fn analyze_screen_crm(app: AppHandle) -> Result<AnalyzeScreenResult, String> {
    // La fenêtre Capture centrale recouvrirait Gmail : on la range, le panneau reste.
    let _ = crate::windows::capture::capture_hide(app.clone());
    let app2 = app.clone();
    tokio::task::spawn_blocking(move || analyze_screen_crm_blocking(&app2))
        .await
        .map_err(|e| e.to_string())?
}

fn analyze_screen_crm_blocking(app: &AppHandle) -> Result<AnalyzeScreenResult, String> {
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Analyse d’écran disponible uniquement sur Windows (OCR local).".into())
    }
    #[cfg(windows)]
    {
        windows_impl::analyze(app)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use crate::windows::capture::CAPTURE_LABEL;
    use crate::windows::panel::PANEL_LABEL;
    use tauri::PhysicalPosition;

    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapDecoder, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    pub fn analyze(app: &AppHandle) -> Result<AnalyzeScreenResult, String> {
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

        let source = app
            .get_webview_window(PANEL_LABEL)
            .or_else(|| app.get_webview_window("main"))
            .ok_or_else(|| "fenêtre MIND introuvable".to_string())?;
        let monitor = source
            .current_monitor()
            .map_err(|e| e.to_string())?
            .or_else(|| source.primary_monitor().ok().flatten())
            .ok_or_else(|| "aucun moniteur".to_string())?;
        let mpos = monitor.position();
        let msize = monitor.size();
        let mw = msize.width;
        let mh = msize.height;
        if mw == 0 || mh == 0 {
            return Err("moniteur invalide".into());
        }

        let mut bgra = capture_bgra(mpos.x, mpos.y, mw, mh)?;
        mask_window(app, PANEL_LABEL, mpos, mw, mh, &mut bgra);
        mask_window(app, CAPTURE_LABEL, mpos, mw, mh, &mut bgra);

        let (ow, oh, scaled) = downscale_max(mw, mh, bgra, 1920);
        let text = ocr_bgra(ow, oh, &scaled)?;
        let draft = parse_crm_from_ocr(&text);
        let found_text = !text.trim().is_empty();
        if !found_text {
            return Err(
                "Aucun texte lisible. Clique dans Gmail (à gauche) puis réessaie.".into(),
            );
        }
        Ok(AnalyzeScreenResult { draft, found_text })
    }

    fn mask_window(
        app: &AppHandle,
        label: &str,
        mon: PhysicalPosition<i32>,
        mw: u32,
        mh: u32,
        bgra: &mut [u8],
    ) {
        let Some(win) = app.get_webview_window(label) else {
            return;
        };
        if !win.is_visible().unwrap_or(false) {
            return;
        }
        let Ok(pos) = win.outer_position() else {
            return;
        };
        let Ok(size) = win.outer_size() else {
            return;
        };
        fill_white(
            bgra,
            mw,
            mh,
            pos.x - mon.x,
            pos.y - mon.y,
            size.width as i32,
            size.height as i32,
        );
    }

    fn fill_white(buf: &mut [u8], mw: u32, mh: u32, x: i32, y: i32, w: i32, h: i32) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = (x.saturating_add(w)).max(0) as u32;
        let y1 = (y.saturating_add(h)).max(0) as u32;
        let x1 = x1.min(mw);
        let y1 = y1.min(mh);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for row in y0..y1 {
            let start = ((row * mw + x0) * 4) as usize;
            let end = ((row * mw + x1) * 4) as usize;
            if end > buf.len() {
                break;
            }
            for px in buf[start..end].chunks_exact_mut(4) {
                px[0] = 255;
                px[1] = 255;
                px[2] = 255;
                px[3] = 255;
            }
        }
    }

    fn downscale_max(w: u32, h: u32, buf: Vec<u8>, max_side: u32) -> (u32, u32, Vec<u8>) {
        let side = w.max(h);
        if side <= max_side {
            return (w, h, buf);
        }
        let scale = max_side as f32 / side as f32;
        let nw = ((w as f32) * scale).round().max(1.0) as u32;
        let nh = ((h as f32) * scale).round().max(1.0) as u32;
        let mut out = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            let sy = (y as f32 / scale) as u32;
            for x in 0..nw {
                let sx = (x as f32 / scale) as u32;
                let si = ((sy.min(h - 1) * w + sx.min(w - 1)) * 4) as usize;
                let di = ((y * nw + x) * 4) as usize;
                out[di..di + 4].copy_from_slice(&buf[si..si + 4]);
            }
        }
        (nw, nh, out)
    }

    fn capture_bgra(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<u8>, String> {
        unsafe {
            let hdc_screen = GetDC(HWND::default());
            if hdc_screen.is_invalid() {
                return Err("GetDC a échoué".into());
            }
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                ReleaseDC(HWND::default(), hdc_screen);
                return Err("CreateCompatibleDC a échoué".into());
            }
            let hbmp = CreateCompatibleBitmap(hdc_screen, w as i32, h as i32);
            if hbmp.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND::default(), hdc_screen);
                return Err("CreateCompatibleBitmap a échoué".into());
            }
            let old = SelectObject(hdc_mem, HGDIOBJ(hbmp.0));
            if let Err(e) = BitBlt(
                hdc_mem,
                0,
                0,
                w as i32,
                h as i32,
                hdc_screen,
                x,
                y,
                SRCCOPY,
            ) {
                SelectObject(hdc_mem, old);
                let _ = DeleteObject(HGDIOBJ(hbmp.0));
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND::default(), hdc_screen);
                return Err(format!("BitBlt: {e}"));
            }
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w * h * 4) as usize];
            let got = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                h,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            SelectObject(hdc_mem, old);
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND::default(), hdc_screen);
            if got == 0 {
                return Err("capture d’écran impossible".into());
            }
            Ok(bgra)
        }
    }

    fn encode_bmp_topdown(width: u32, height: u32, bgra: &[u8]) -> Vec<u8> {
        let pixel_size = (width * height * 4) as usize;
        let file_size = 54 + pixel_size;
        let mut out = Vec::with_capacity(file_size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        out.extend_from_slice(&(-(height as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(pixel_size as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&bgra[..pixel_size.min(bgra.len())]);
        out
    }

    fn ocr_bgra(width: u32, height: u32, bgra: &[u8]) -> Result<String, String> {
        let bmp = encode_bmp_topdown(width, height, bgra);
        let stream = InMemoryRandomAccessStream::new().map_err(|e| e.to_string())?;
        let writer = DataWriter::CreateDataWriter(&stream).map_err(|e| e.to_string())?;
        writer.WriteBytes(&bmp).map_err(|e| e.to_string())?;
        writer
            .StoreAsync()
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        writer
            .FlushAsync()
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        drop(writer);
        stream.Seek(0).map_err(|e| e.to_string())?;
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        let software: SoftwareBitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;

        let engine = ocr_engine()?;
        let result = engine
            .RecognizeAsync(&software)
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        Ok(result.Text().map_err(|e| e.to_string())?.to_string())
    }

    fn ocr_engine() -> Result<OcrEngine, String> {
        if let Ok(lang) = Language::CreateLanguage(&HSTRING::from("fr-FR")) {
            if OcrEngine::IsLanguageSupported(&lang).unwrap_or_default().as_bool() {
                if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
                    return Ok(engine);
                }
            }
        }
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|e| {
            format!("OCR Windows indisponible ({e}). Installe une langue de reconnaissance.")
        })
    }
}
