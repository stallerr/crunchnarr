import type { ReactNode } from 'react';

export interface Avatar {
  id: number;
  svg: ReactNode;
  name: string;
  rgb: string;
}

export const avatars: Avatar[] = [
  {
    id: 1,
    name: 'Blaze',
    rgb: '255, 0, 91',
    svg: (
      <svg
        aria-label="Blaze"
        fill="none"
        height="40"
        role="img"
        viewBox="0 0 36 36"
        width="40"
        xmlns="http://www.w3.org/2000/svg"
      >
        <title>Blaze</title>
        <mask height="36" id="av1" maskUnits="userSpaceOnUse" width="36" x="0" y="0">
          <rect fill="#FFFFFF" height="36" rx="72" width="36" />
        </mask>
        <g mask="url(#av1)">
          <rect fill="#ff005b" height="36" width="36" />
          <rect
            fill="#ffb238"
            height="36"
            rx="6"
            transform="translate(9 -5) rotate(219 18 18) scale(1)"
            width="36"
            x="0"
            y="0"
          />
          <g transform="translate(4.5 -4) rotate(9 18 18)">
            <path d="M15 19c2 1 4 1 6 0" fill="none" stroke="#000000" strokeLinecap="round" />
            <rect fill="#000000" height="2" rx="1" stroke="none" width="1.5" x="10" y="14" />
            <rect fill="#000000" height="2" rx="1" stroke="none" width="1.5" x="24" y="14" />
          </g>
        </g>
      </svg>
    ),
  },
  {
    id: 2,
    name: 'Shadow',
    rgb: '255, 125, 16',
    svg: (
      <svg
        aria-label="Shadow"
        fill="none"
        height="40"
        role="img"
        viewBox="0 0 36 36"
        width="40"
        xmlns="http://www.w3.org/2000/svg"
      >
        <title>Shadow</title>
        <mask height="36" id="av2" maskUnits="userSpaceOnUse" width="36" x="0" y="0">
          <rect fill="#FFFFFF" height="36" rx="72" width="36" />
        </mask>
        <g mask="url(#av2)">
          <rect fill="#ff7d10" height="36" width="36" />
          <rect
            fill="#0a0310"
            height="36"
            rx="6"
            transform="translate(5 -1) rotate(55 18 18) scale(1.1)"
            width="36"
            x="0"
            y="0"
          />
          <g transform="translate(7 -6) rotate(-5 18 18)">
            <path d="M15 20c2 1 4 1 6 0" fill="none" stroke="#FFFFFF" strokeLinecap="round" />
            <rect fill="#FFFFFF" height="2" rx="1" stroke="none" width="1.5" x="14" y="14" />
            <rect fill="#FFFFFF" height="2" rx="1" stroke="none" width="1.5" x="20" y="14" />
          </g>
        </g>
      </svg>
    ),
  },
  {
    id: 3,
    name: 'Ember',
    rgb: '255, 0, 91',
    svg: (
      <svg
        aria-label="Ember"
        fill="none"
        height="40"
        role="img"
        viewBox="0 0 36 36"
        width="40"
        xmlns="http://www.w3.org/2000/svg"
      >
        <title>Ember</title>
        <mask height="36" id="av3" maskUnits="userSpaceOnUse" width="36" x="0" y="0">
          <rect fill="#FFFFFF" height="36" rx="72" width="36" />
        </mask>
        <g mask="url(#av3)">
          <rect fill="#0a0310" height="36" width="36" />
          <rect
            fill="#ff005b"
            height="36"
            rx="36"
            transform="translate(-3 7) rotate(227 18 18) scale(1.2)"
            width="36"
            x="0"
            y="0"
          />
          <g transform="translate(-3 3.5) rotate(7 18 18)">
            <path d="M13,21 a1,0.75 0 0,0 10,0" fill="#FFFFFF" />
            <rect fill="#FFFFFF" height="2" rx="1" stroke="none" width="1.5" x="12" y="14" />
            <rect fill="#FFFFFF" height="2" rx="1" stroke="none" width="1.5" x="22" y="14" />
          </g>
        </g>
      </svg>
    ),
  },
  {
    id: 4,
    name: 'Sprout',
    rgb: '137, 252, 179',
    svg: (
      <svg
        aria-label="Sprout"
        fill="none"
        height="40"
        role="img"
        viewBox="0 0 36 36"
        width="40"
        xmlns="http://www.w3.org/2000/svg"
      >
        <title>Sprout</title>
        <mask height="36" id="av4" maskUnits="userSpaceOnUse" width="36" x="0" y="0">
          <rect fill="#FFFFFF" height="36" rx="72" width="36" />
        </mask>
        <g mask="url(#av4)">
          <rect fill="#d8fcb3" height="36" width="36" />
          <rect
            fill="#89fcb3"
            height="36"
            rx="6"
            transform="translate(9 -5) rotate(219 18 18) scale(1)"
            width="36"
            x="0"
            y="0"
          />
          <g transform="translate(4.5 -4) rotate(9 18 18)">
            <path d="M15 19c2 1 4 1 6 0" fill="none" stroke="#000000" strokeLinecap="round" />
            <rect fill="#000000" height="2" rx="1" stroke="none" width="1.5" x="10" y="14" />
            <rect fill="#000000" height="2" rx="1" stroke="none" width="1.5" x="24" y="14" />
          </g>
        </g>
      </svg>
    ),
  },
];

export const getAvatarById = (id: number): Avatar =>
  avatars.find((a) => a.id === id) ?? avatars[0];
