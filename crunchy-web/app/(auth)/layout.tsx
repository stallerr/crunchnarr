import { ReactNode } from 'react';
import PixelBlast from "@/components/ui/pixel-blast.tsx";
import {Spotlight} from "@/components/ui/spotlight.tsx";

export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <div className="relative flex min-h-screen items-center justify-center bg-background p-4">
          {/*<Spotlight*/}
          {/*    className='bg-primary blur-2xl'*/}
          {/*    size={200}*/}
          {/*    springOptions={{*/}
          {/*        bounce: 0.5,*/}
          {/*        duration: 5,*/}
          {/*    }}*/}
          {/*/>*/}
      <div className='absolute inset-4 overflow-hidden rounded-2xl'>

          {/*<PixelBlast*/}
        {/*    variant='circle'*/}
        {/*    pixelSize={8}*/}
        {/*    color='#F47521'*/}
        {/*    patternScale={1.5}*/}
        {/*    patternDensity={1}*/}
        {/*    pixelSizeJitter={0}*/}
        {/*    liquid={false}*/}
        {/*    speed={0.5}*/}
        {/*    edgeFade={0.15}*/}
        {/*    transparent*/}
        {/*/>*/}
        <div className="absolute inset-4 bg-background opacity-0"></div>
      </div>

      {children}
    </div>
  );
}
