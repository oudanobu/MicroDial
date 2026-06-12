/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from 'react';
import { WatchFace } from './components/WatchFace';
import { SensorControls } from './components/SensorControls';
import { SensorData } from './types';

export default function App() {
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

  return (
    <div className="min-h-screen bg-slate-50 flex items-center justify-center p-8 font-sans">
      <div className="max-w-4xl w-full flex flex-col lg:flex-row items-center justify-center gap-16">
        
        {/* Hardware / Sensor Panel */}
        <div className="w-full lg:w-auto flex justify-center">
          <SensorControls data={sensorData} onChange={setSensorData} />
        </div>

        {/* Device Emulator Frame */}
        <div className="flex flex-col items-center">
          <div className="bg-slate-200 text-slate-500 text-xs font-medium uppercase tracking-widest px-4 py-1.5 rounded-full mb-6">
            Display Surface Out
          </div>
          <WatchFace sensorData={sensorData} />
          
          <div className="mt-8 text-center">
            <h1 className="text-lg font-bold text-slate-900 tracking-tight">Sports Watch Renderer</h1>
            <p className="text-sm text-slate-500 mt-1 max-w-sm">
              Rendering dynamic sensor values via simulated low-memory constrained engine buffer.
            </p>
          </div>
        </div>

      </div>
    </div>
  );
}

