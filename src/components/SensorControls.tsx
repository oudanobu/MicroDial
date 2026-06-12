import { SensorData } from '../types';
import { Language, translations } from '../lib/translations';

interface SensorControlsProps {
  data: SensorData;
  onChange: (data: SensorData) => void;
  lang: Language;
}

export function SensorControls({ data, onChange, lang }: SensorControlsProps) {
  const handleChange = (key: keyof SensorData, value: number) => {
    onChange({ ...data, [key]: value });
  };

  const t = translations[lang];

  return (
    <div className="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col gap-6 shadow-sm w-full max-w-sm">
      <div>
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wider mb-4">{t.hardwareProvider}</h2>
        <div className="flex flex-col gap-5">
          {/* Step Counter */}
          <div>
            <div className="flex justify-between mb-1">
              <label className="text-xs font-medium text-slate-600">{lang === 'zh' ? '实时计步值' : 'Step Count'}</label>
              <span className="text-xs font-mono text-slate-900">{data.steps}</span>
            </div>
            <input
              type="range"
              min="0"
              max="20000"
              value={data.steps}
              onChange={(e) => handleChange('steps', parseInt(e.target.value))}
              className="w-full accent-slate-900"
            />
          </div>

          {/* Barometer */}
          <div>
            <div className="flex justify-between mb-1">
              <label className="text-xs font-medium text-slate-600">{lang === 'zh' ? '高度气压值 (hPa)' : 'Barometer (hPa)'}</label>
              <span className="text-xs font-mono text-slate-900">{data.pressure.toFixed(1)}</span>
            </div>
            <input
              type="range"
              min="900"
              max="1100"
              step="0.1"
              value={data.pressure}
              onChange={(e) => handleChange('pressure', parseFloat(e.target.value))}
              className="w-full accent-slate-900"
            />
          </div>

          {/* Compass Heading */}
          <div>
            <div className="flex justify-between mb-1">
              <label className="text-xs font-medium text-slate-600">{lang === 'zh' ? '传感器指南航向角' : 'Compass Heading'}</label>
              <span className="text-xs font-mono text-slate-900">{data.heading}°</span>
            </div>
            <input
              type="range"
              min="0"
              max="360"
              value={data.heading}
              onChange={(e) => handleChange('heading', parseInt(e.target.value))}
              className="w-full accent-slate-900"
            />
          </div>

          <div className="pt-4 border-t border-slate-100 flex flex-col gap-2">
            <div className="flex justify-between text-xs">
              <span className="text-slate-500">{t.pinnedMemoryStatus}</span>
              <span className="font-mono text-emerald-600 font-medium">{t.pinnedMemoryActive}</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-slate-500">{t.bufferAllocation}</span>
              <span className="font-mono text-slate-900 font-medium">14.2 MB / 512.0 MB</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
