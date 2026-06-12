import { SensorData } from '../types';

interface SensorControlsProps {
  data: SensorData;
  onChange: (data: SensorData) => void;
}

export function SensorControls({ data, onChange }: SensorControlsProps) {
  const handleChange = (key: keyof SensorData, value: number) => {
    onChange({ ...data, [key]: value });
  };

  return (
    <div className="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col gap-6 shadow-sm w-full max-w-sm">
      <div>
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wider mb-4">Hardware Data Provider</h2>
        <div className="flex flex-col gap-5">
          {/* Step Counter */}
          <div>
            <div className="flex justify-between mb-1">
              <label className="text-xs font-medium text-slate-600">Step Count</label>
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
              <label className="text-xs font-medium text-slate-600">Barometer (hPa)</label>
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
              <label className="text-xs font-medium text-slate-600">Compass Heading</label>
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
              <span className="text-slate-500">Pinned Memory Status</span>
              <span className="font-mono text-emerald-600 font-medium">Active</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-slate-500">Buffer Allocation</span>
              <span className="font-mono text-slate-900 font-medium">14.2 MB / 512.0 MB</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
