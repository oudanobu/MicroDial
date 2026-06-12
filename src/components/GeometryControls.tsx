import { Language, translations } from '../lib/translations';
import { Eye, ShieldAlert, Monitor, Spline, MoveLeft, Terminal } from 'lucide-react';

export interface GeometrySettings {
  shape: 'Square' | 'Round';
  resolution: 240 | 320;
  dragStartX: number;
  dragOffsetX: number;
  isDragging: boolean;
  densityScale: number;
}

interface GeometryControlsProps {
  settings: GeometrySettings;
  onChange: (settings: GeometrySettings) => void;
  lang: Language;
}

export function GeometryControls({ settings, onChange, lang }: GeometryControlsProps) {
  const t = translations[lang];

  const handleShapeChange = (shape: 'Square' | 'Round') => {
    onChange({ ...settings, shape });
  };

  const handleResolutionChange = (res: 240 | 320) => {
    const scale = res / 320.0;
    onChange({
      ...settings,
      resolution: res,
      densityScale: scale,
      dragOffsetX: Math.max(settings.dragOffsetX, -res)
    });
  };

  const handleDragOffsetChange = (val: number) => {
    onChange({ ...settings, dragOffsetX: val });
  };

  // Generate a mini 20x20 preview grid at the top-left extreme (x: 0~14, y: 0~14) to illustrate the clipping
  const loupePixels = [];
  const r = settings.resolution / 2;
  const scaleRatio = settings.resolution / 14; // compress 14 cells into the semi-radius viewport for visual clarity

  for (let y = 0; y < 14; y++) {
    for (let x = 0; x < 14; x++) {
      // Scale cell coordinates to actual screen coordinates
      const actX = Math.round(x * scaleRatio);
      const actY = Math.round(y * scaleRatio);

      let visible = true;
      if (settings.shape === 'Round') {
        const dx = actX - r;
        const dy = actY - r;
        visible = dx * dx + dy * dy <= r * r;
      } else {
        visible = actX < settings.resolution && actY < settings.resolution;
      }
      loupePixels.push({ x, y, actX, actY, visible });
    }
  }

  return (
    <div className="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col gap-6 shadow-sm w-full max-w-sm">
      
      {/* Target Geometry Section */}
      <div>
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wider mb-4 flex items-center gap-2">
          <Monitor className="w-4 h-4 text-sky-600" />
          {t.geometryTitle}
        </h2>
        
        <div className="flex flex-col gap-4">
          
          {/* Screen Shape Select */}
          <div>
            <label className="text-xs font-semibold text-slate-600 block mb-2">{t.screenShape}</label>
            <div className="grid grid-cols-2 gap-2">
              <button
                onClick={() => handleShapeChange('Square')}
                className={`py-2 px-3 text-xs font-semibold rounded-lg border transition ${
                  settings.shape === 'Square'
                    ? 'bg-slate-900 border-slate-900 text-white'
                    : 'bg-slate-50 border-slate-200 text-slate-700 hover:bg-slate-100'
                }`}
              >
                {t.squareShape}
              </button>
              <button
                onClick={() => handleShapeChange('Round')}
                className={`py-2 px-3 text-xs font-semibold rounded-lg border transition ${
                  settings.shape === 'Round'
                    ? 'bg-slate-900 border-slate-900 text-white'
                    : 'bg-slate-50 border-slate-200 text-slate-700 hover:bg-slate-100'
                }`}
              >
                {t.roundShape}
              </button>
            </div>
          </div>

          {/* Screen Resolution */}
          <div>
            <label className="text-xs font-semibold text-slate-600 block mb-2">{t.resolution}</label>
            <div className="grid grid-cols-2 gap-2">
              <button
                onClick={() => handleResolutionChange(240)}
                className={`py-2 px-3 text-xs font-mono font-bold rounded-lg border transition ${
                  settings.resolution === 240
                    ? 'bg-slate-900 border-slate-900 text-white'
                    : 'bg-slate-50 border-slate-200 text-slate-700 hover:bg-slate-100'
                }`}
              >
                240 × 240 px
              </button>
              <button
                onClick={() => handleResolutionChange(320)}
                className={`py-2 px-3 text-xs font-mono font-bold rounded-lg border transition ${
                  settings.resolution === 320
                    ? 'bg-slate-900 border-slate-900 text-white'
                    : 'bg-slate-50 border-slate-200 text-slate-700 hover:bg-slate-100'
                }`}
              >
                320 × 320 px
              </button>
            </div>
          </div>

          {/* Density Scale Indicators */}
          <div className="flex justify-between items-center bg-slate-50 border border-slate-100 rounded-lg p-3 text-xs">
            <span className="text-slate-500 font-medium">{t.densityScale}</span>
            <span className="font-mono bg-slate-200 border border-slate-300 text-slate-800 px-2 py-0.5 rounded font-bold">
              {settings.densityScale.toFixed(2)}x
            </span>
          </div>

        </div>
      </div>

      {/* Swipe Viewport State Machine Section */}
      <div className="pt-4 border-t border-slate-100">
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wider mb-4 flex items-center gap-2">
          <Spline className="w-4 h-4 text-emerald-600" />
          {t.gestureTitle}
        </h2>

        <div className="flex flex-col gap-4">
          
          {/* Slider for sliding App Drawer */}
          <div>
            <div className="flex justify-between mb-1.5 text-xs">
              <span className="text-slate-600 font-semibold flex items-center gap-1">
                <MoveLeft className="w-3.5 h-3.5" />
                {t.swipeProgress}
              </span>
              <span className="font-mono text-slate-900 font-bold">{settings.dragOffsetX}px</span>
            </div>
            
            <input
              type="range"
              min={-settings.resolution}
              max={0}
              value={settings.dragOffsetX}
              onChange={(e) => handleDragOffsetChange(parseInt(e.target.value))}
              className="w-full accent-emerald-600"
            />
          </div>

          {/* Status badge alerts */}
          <div className="flex items-center justify-between text-xs bg-slate-50 p-2.5 rounded-lg border border-slate-100">
            <span className="text-slate-500 font-medium">State Machine</span>
            <span className={`font-mono px-2 py-0.5 rounded text-[10px] font-bold ${
              settings.dragOffsetX < 0 
                ? 'bg-emerald-100 text-emerald-700 border border-emerald-200' 
                : 'bg-slate-200 text-slate-600 border border-slate-300'
            }`}>
              {settings.dragOffsetX < 0 ? t.dragActive : t.dragIdle}
            </span>
          </div>

          {settings.dragOffsetX < 0 && (
            <div className="flex items-center gap-2 text-xs text-amber-700 bg-amber-50 border border-amber-200 p-2.5 rounded-lg">
              <Terminal className="w-4 h-4 shrink-0" />
              <span>{t.activeDrawer}</span>
            </div>
          )}

        </div>
      </div>

      {/* Loupe Pixel Raster Filter */}
      <div className="pt-4 border-t border-slate-100">
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wider mb-2 flex items-center gap-2">
          <Eye className="w-4 h-4 text-amber-500" />
          {t.borderCheck}
        </h2>
        <p className="text-[11px] text-slate-400 mb-3 leading-normal">
          {t.borderCheckDesc}
        </p>

        <div className="flex flex-col items-center">
          {/* The Grid Map container */}
          <div className="grid grid-cols-14 gap-1 p-2 bg-slate-950 rounded-xl border border-slate-800">
            {loupePixels.map((p, idx) => (
              <div
                key={idx}
                title={`Pixel (${p.actX}, ${p.actY}): ${p.visible ? 'Visible' : 'Clipped/Black'}`}
                className={`w-3.5 h-3.5 rounded-sm transition-all duration-200 ${
                  p.visible
                    ? 'bg-sky-500 hover:bg-sky-400 cursor-help'
                    : 'bg-red-950/40 border border-red-900/30 text-transparent opacity-40 hover:bg-red-900/50'
                }`}
              />
            ))}
          </div>

          <div className="flex justify-between w-full mt-2.5 text-[10px] font-mono text-slate-400 px-1">
            <span className="flex items-center gap-1">
              <span className="inline-block w-2.5 h-2.5 bg-sky-500 rounded-sm" />
              Visible Area
            </span>
            <span className="flex items-center gap-1">
              <span className="inline-block w-2.5 h-2.5 bg-red-950 border border-red-900 rounded-sm" />
              Clipped (0x0000)
            </span>
          </div>
        </div>
      </div>

    </div>
  );
}
