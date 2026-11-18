// What the showcase highlights at each step and what its caption says.
export type Spot = { spot: string[]; caption: string; off?: boolean };

const ALL: Spot = { spot: ['all'], caption: 'Point at an item to see where it shows up.' };

const C1: Spot[] = [
  { spot: ['kind', 'name', 'voiceicon'], caption: 'The game sets the name after "Playing" and the small icon next to your name in voice channels.' },
  { spot: ['details', 'state'], caption: '"Details" is the first line under the name. "State" is the second.' },
  { spot: ['all'], caption: 'Once it is on, this is what people see on your profile.' },
];

const C2: Spot[] = [
  { spot: ['kind', 'name'], caption: 'The name you type becomes the name after "Playing".' },
  { spot: ['voiceicon'], caption: 'The app icon is the small square next to your name in voice channels.' },
  { spot: ['big', 'badge'], caption: 'Your pictures fill the large image and the round badge on it.' },
  { spot: ['name', 'voiceicon'], caption: 'The ID tells Discord whose name and icon to show.' },
];

export const REF: Record<string, Spot> = {
  large: { spot: ['big'], caption: 'The large image is the big square on the card.' },
  small: { spot: ['badge'], caption: 'The small image is the round badge on its corner.' },
  key: { spot: ['big', 'badge'], caption: 'Both pictures take an asset key or a link.' },
  hover: { spot: ['big', 'badge'], caption: 'Hover text appears when someone points at a picture.' },
  buttons: { spot: ['buttons'], caption: 'Buttons sit at the bottom of the card.' },
  party: { spot: ['state'], caption: 'The party size is added to the end of the "State" line.' },
  timer: { spot: ['timer'], caption: 'The timer runs under your text.' },
  member: { spot: ['member'], caption: 'This line sits under your name in the member list.' },
};

export function spotFor(chapter: number, step: number, hot: string | null): Spot {
  if (chapter === 1) return C1[step - 1] ?? ALL;
  if (chapter === 2) return C2[step - 1] ?? ALL;
  if (chapter === 3) return (hot && REF[hot]) || ALL;
  return { spot: [], off: true, caption: 'If your profile looks empty in Discord, go through the list on the left.' };
}
