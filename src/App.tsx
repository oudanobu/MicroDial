/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from 'react';
import { WatchFace } from './components/WatchFace';
import { SensorControls } from './components/SensorControls';
import { SensorData } from './types';
import { Language, translations } from './lib/translations';
import { Globe, Cpu, CheckCircle } from 'lucide-react';

export default function App() {
  const [lang, setLang] = useState<Language>('zh');

  // Initialize with the exact mock provider data from the Rust unit test
  const [sensorData, setSensorData] = useState<SensorData>({
    steps: 8432,
    heading: 180.0,
    longitude: 116.4074,
    latitude: 39.9042, // Beijing Tiananmen coords
    pressure: 1011.5,
    weather: {
      temperature: 24,
      conditionCode: 1,
      locationId: 110000,
      humidity: 50
    }
  });

  const t = translations[lang];

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col justify-between font-sans">
      
      {/* Top Navigation / Header */}
      <header className="border-b border-slate-200 bg-white shadow-sm/50 px-6 py-4">
        <div className="max-w-6xl mx-auto flex justify-between items-center">
          <div className="flex items-center gap-3">
            <Cpu className="w-6 h-6 text-slate-800" />
            <span className="font-mono font-bold tracking-tight text-slate-900 text-lg">
              ChronOxide <span className="text-xs text-sky-600 bg-sky-50 px-2 py-0.5 rounded border border-sky-100 font-medium ml-1">Rust Kernel</span>
            </span>
          </div>

          <button
            onClick={() => setLang(lang === 'zh' ? 'en' : 'zh')}
            className="flex items-center gap-2 text-xs font-semibold bg-slate-100 hover:bg-slate-200 text-slate-700 px-3.5 py-1.5 rounded-lg transition"
          >
            <Globe className="w-3.5 h-3.5" />
            <span>{t.toggleLanguage}</span>
          </button>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 max-w-6xl w-full mx-auto p-6 md:p-12 flex flex-col items-center justify-center gap-8 md:gap-16">
        <div className="text-center max-w-2xl">
          <h1 className="text-3xl md:text-4xl font-extrabold text-slate-900 tracking-tight">
            {t.brandName}
          </h1>
          <p className="text-sm md:text-base text-slate-500 mt-3 max-w-xl mx-auto">
            {t.subtitle}
          </p>
          <div className="mt-4 flex flex-wrap justify-center gap-3">
            <span className="text-xs font-mono font-medium text-emerald-700 bg-emerald-50 border border-emerald-200 px-2.5 py-1 rounded-md flex items-center gap-1">
              <CheckCircle className="w-3 h-3" />
              {t.memoryMode}
            </span>
            <span className="text-xs font-mono font-medium text-slate-700 bg-slate-100 border border-slate-200 px-2.5 py-1 rounded-md">
              {t.compileStatus}
            </span>
          </div>
        </div>

        <div className="w-full flex flex-col lg:flex-row items-center justify-center gap-12 lg:gap-20">
          {/* Hardware / Sensor Panel */}
          <div className="w-full lg:w-auto flex justify-center">
            <SensorControls data={sensorData} onChange={setSensorData} lang={lang} />
          </div>

          {/* Device Emulator Frame */}
          <div className="flex flex-col items-center">
            <div className="bg-slate-200 text-slate-600 text-[10px] font-mono font-semibold uppercase tracking-wider px-3 py-1 rounded-md mb-6 border border-slate-300">
              {t.displaySurface}
            </div>
            <WatchFace sensorData={sensorData} lang={lang} />
            
            <div className="mt-8 text-center">
              <h2 className="text-md font-bold text-slate-800 tracking-tight">
                {t.watchfaceName}
              </h2>
              <p className="text-xs text-slate-400 mt-1.5 max-w-xs">
                {t.uncompressedWarning}
              </p>
            </div>
          </div>
        </div>
      </main>

      {/* Embedded Watch Architecture Status Footer */}
      <footer className="bg-white border-t border-slate-200 py-4 px-6 text-center text-[11px] font-mono text-slate-400">
        ChronOxide Co., Ltd. &bull; {lang === 'zh' ? '在 512MB RAM 下实现 15MB 极限内存开销守护' : 'Safeguarding 15MB RAM Overhead on 512MB Systems'}
      </footer>

    </div>
  );
}

