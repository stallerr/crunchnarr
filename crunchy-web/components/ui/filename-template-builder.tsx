import {useState, useRef, useCallback, useEffect, DragEvent} from "react";

type BlockType = "variable" | "directory" | "separator";

interface Block {
    id: string;
    label: string;
    type: BlockType;
}

interface TemplateBlock extends Block {
    uid: string;
}

interface DragSourcePalette {
    from: "palette";
    block: Block;
}

interface DragSourceTemplate {
    from: "template";
    idx: number;
}

type DragSource = DragSourcePalette | DragSourceTemplate;

interface BlockBadgeProps {
    block: Block;
    onRemove?: () => void;
    draggable?: boolean;
    onDragStart?: (e: DragEvent<HTMLDivElement>) => void;
    onDragOver?: (e: DragEvent<HTMLDivElement>) => void;
    onDrop?: (e: DragEvent<HTMLDivElement>) => void;
    isDragging?: boolean;
    isOver?: boolean;
}

interface FilenameTemplateBuilderProps {
    value?: string;
    onChange?: (value: string) => void;
}

const VARIABLE_BLOCKS: Block[] = [
    {id: "series", label: "{series}", type: "variable"},
    {id: "season", label: "{season:02}", type: "variable"},
    {id: "season-plain", label: "{season}", type: "variable"},
    {id: "season-title", label: "{season_title}", type: "variable"},
    {id: "episode", label: "{episode:02}", type: "variable"},
    {id: "episode-plain", label: "{episode}", type: "variable"},
    {id: "title", label: "{title}", type: "variable"},
    {id: "quality", label: "{quality}", type: "variable"},
    {id: "audio", label: "{audio}", type: "variable"},
    {id: "year", label: "{year}", type: "variable"},
];

const VAR_PATTERN = /(\{series\}|\{season:02\}|\{season\}|\{season_title\}|\{episode:02\}|\{episode\}|\{title\}|\{quality\}|\{audio\}|\{year\})/;

function parseTemplateString(value: string): TemplateBlock[] {
    if (!value) return [];

    const blocks: TemplateBlock[] = [];
    const parts = value.split(VAR_PATTERN);

    for (const part of parts) {
        if (!part) continue;

        const matchedVar = VARIABLE_BLOCKS.find(v => v.label === part);
        if (matchedVar) {
            blocks.push({uid: uid(), id: matchedVar.id, label: matchedVar.label, type: "variable"});
        } else {
            blocks.push({uid: uid(), id: `sep-${uid()}`, label: part, type: "separator"});
        }
    }

    return blocks;
}

const SEPARATOR_PRESETS = [" - ", " ", ".", "_", "x"];

const uid = () => Math.random().toString(36).slice(2, 9);

const colors: Record<BlockType, string> = {
    variable: "bg-violet-500/20 text-violet-300 border-violet-500/40",
    directory: "bg-amber-500/20 text-amber-300 border-amber-500/40",
    separator: "bg-slate-500/20 text-slate-300 border-slate-500/40",
};

const icons: Record<BlockType, React.ReactNode> = {
    variable: (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"
             strokeLinecap="round" strokeLinejoin="round">
            <path d="M4 7V4h16v3"/>
            <path d="M9 20h6"/>
            <path d="M12 4v16"/>
        </svg>
    ),
    directory: (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"
             strokeLinecap="round" strokeLinejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
    ),
    separator: (
        <></>
        // <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"
        //      strokeLinecap="round" strokeLinejoin="round">
        //     <path d="M18 6L6 18"/>
        // </svg>
    ),
};

const BlockBadge = ({block, onRemove, draggable, onDragStart, onDragOver, onDrop, isDragging, isOver}: BlockBadgeProps) => {
    return (
        <div
            draggable={draggable}
            onDragStart={onDragStart}
            onDragOver={onDragOver}
            onDrop={onDrop}
            className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-sm font-medium
        ${colors[block.type]}
        ${draggable ? "cursor-grab active:cursor-grabbing" : ""}
        ${isDragging ? "opacity-30 scale-95" : ""}
        ${isOver ? "ring-2 ring-violet-400 ring-offset-1 ring-offset-zinc-900" : ""}
        transition-all duration-150`}
        >
            <span className="opacity-70 shrink-0">{icons[block.type]}</span>
            <span className="whitespace-nowrap select-none">
        {block.type === "directory" ? `/${block.label}/` : block.label}
      </span>
            {onRemove && (
                <button
                    onClick={(e) => {
                        e.stopPropagation();
                        onRemove();
                    }}
                    className="ml-0.5 opacity-50 hover:opacity-100 transition-opacity rounded-full hover:bg-white/10 p-0.5"
                >
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3"
                         strokeLinecap="round">
                        <path d="M18 6L6 18M6 6l12 12"/>
                    </svg>
                </button>
            )}
        </div>
    );
};

export default function FilenameTemplateBuilder({value, onChange}: FilenameTemplateBuilderProps) {
    const [template, setTemplate] = useState<TemplateBlock[]>(() =>
        parseTemplateString(value ?? "")
    );

    const [dragSource, setDragSource] = useState<DragSource | null>(null);
    const [dragOverIdx, setDragOverIdx] = useState<number | null>(null);
    const [showDirInput, setShowDirInput] = useState(false);
    const [showSepInput, setShowSepInput] = useState(false);
    const [dirValue, setDirValue] = useState("");
    const [sepValue, setSepValue] = useState("");
    const dropZoneRef = useRef<HTMLDivElement>(null);

    const getTemplateString = useCallback(() => {
        return template.map(b => {
            if (b.type === "directory") return `${b.label}/`;
            return b.label;
        }).join("");
    }, [template]);

    useEffect(() => {
        onChange?.(getTemplateString());
    }, [template, onChange, getTemplateString]);

    const onPaletteDragStart = (e: DragEvent<HTMLDivElement>, block: Block) => {
        setDragSource({from: "palette", block});
        e.dataTransfer.effectAllowed = "copy";
    };

    const onTemplateDragStart = (e: DragEvent<HTMLDivElement>, idx: number) => {
        setDragSource({from: "template", idx});
        e.dataTransfer.effectAllowed = "move";
    };

    const onTemplateDragOver = (e: DragEvent<HTMLDivElement>, idx: number) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = dragSource?.from === "palette" ? "copy" : "move";
        setDragOverIdx(idx);
    };

    const onTemplateDrop = (e: DragEvent<HTMLDivElement>, idx: number) => {
        e.preventDefault();
        e.stopPropagation();
        if (!dragSource) return;

        setTemplate(prev => {
            const next = [...prev];
            if (dragSource.from === "palette") {
                const newBlock: TemplateBlock = {...dragSource.block, uid: uid()};
                next.splice(idx, 0, newBlock);
            } else if (dragSource.from === "template") {
                const [moved] = next.splice(dragSource.idx, 1);
                const insertIdx = idx > dragSource.idx ? idx - 1 : idx;
                next.splice(insertIdx, 0, moved);
            }
            return next;
        });
        setDragSource(null);
        setDragOverIdx(null);
    };

    const onDropZoneDragOver = (e: DragEvent<HTMLDivElement>) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = dragSource?.from === "palette" ? "copy" : "move";
        if (dragSource?.from === "palette") setDragOverIdx(template.length);
    };

    const onDropZoneDrop = (e: DragEvent<HTMLDivElement>) => {
        e.preventDefault();
        if (!dragSource) return;
        if (dragSource.from === "palette") {
            const newBlock: TemplateBlock = {...dragSource.block, uid: uid()};
            setTemplate(prev => [...prev, newBlock]);
        }
        setDragSource(null);
        setDragOverIdx(null);
    };

    const onDragEnd = () => {
        setDragSource(null);
        setDragOverIdx(null);
    };

    const removeBlock = (idx: number) => {
        setTemplate(prev => prev.filter((_, i) => i !== idx));
    };

    const addDirectory = () => {
        if (!dirValue.trim()) return;
        setTemplate(prev => [...prev, {uid: uid(), id: `dir-${uid()}`, label: dirValue.trim(), type: "directory"}]);
        setDirValue("");
        setShowDirInput(false);
    };

    const addSeparator = (val?: string) => {
        const v = val ?? sepValue;
        if (!v) return;
        setTemplate(prev => [...prev, {uid: uid(), id: `sep-${uid()}`, label: v, type: "separator"}]);
        setSepValue("");
        setShowSepInput(false);
    };

    return (
        <div className="flex items-start" onDragEnd={onDragEnd}>
            <div className="w-full space-y-6">
                {/* Variable Blocks Palette */}
                <div className="space-y-2.5">
                    <label className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Variable
                        Blocks</label>
                    <div className="flex flex-wrap gap-2">
                        {VARIABLE_BLOCKS.map(b => (
                            <div key={b.id} draggable onDragStart={(e) => onPaletteDragStart(e, b)}>
                                <BlockBadge block={b}/>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Drop zone / Template input */}
                <div className="space-y-2.5">
                    <label className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Template</label>
                    <div
                        ref={dropZoneRef}
                        onDragOver={onDropZoneDragOver}
                        onDrop={onDropZoneDrop}
                        className={`min-h-14 w-full rounded-lg border-2 border-dashed px-3 py-2.5 flex flex-wrap items-center gap-1.5
              transition-colors duration-150
              ${dragSource ? "border-violet-500/50 bg-violet-500/5" : "border-zinc-700 bg-zinc-900/50"}
              ${template.length === 0 ? "justify-center" : ""}`}
                    >
                        {template.length === 0 && (
                            <span
                                className="text-zinc-600 text-sm select-none">Drop blocks here to build your template…</span>
                        )}
                        {template.map((block, idx) => (
                            <div key={block.uid} className="flex items-center">
                                {dragOverIdx === idx && (
                                    <div className="w-0.5 h-7 bg-violet-400 rounded-full mr-1 animate-pulse"/>
                                )}
                                <BlockBadge
                                    block={block}
                                    onRemove={() => removeBlock(idx)}
                                    draggable
                                    onDragStart={(e) => onTemplateDragStart(e, idx)}
                                    onDragOver={(e) => onTemplateDragOver(e, idx)}
                                    onDrop={(e) => onTemplateDrop(e, idx)}
                                    isDragging={dragSource?.from === "template" && dragSource.idx === idx}
                                    isOver={dragOverIdx === idx}
                                />
                            </div>
                        ))}
                        {dragOverIdx === template.length && (
                            <div className="w-0.5 h-7 bg-violet-400 rounded-full ml-1 animate-pulse"/>
                        )}
                    </div>
                </div>

                {/* Add Buttons */}
                <div className="flex flex-wrap gap-2">
                    {/* Add Directory */}
                    <div className="relative">
                        <button
                            onClick={() => {
                                setShowDirInput(!showDirInput);
                                setShowSepInput(false);
                            }}
                            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-amber-500/10 text-amber-300 border border-amber-500/30 text-sm font-medium hover:bg-amber-500/20 transition-colors"
                        >
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                                 strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
                            </svg>
                            Add Directory
                        </button>
                        {showDirInput && (
                            <div
                                className="absolute top-full mt-2 left-0 z-10 bg-zinc-800 border border-zinc-700 rounded-lg p-3 shadow-xl min-w-60">
                                <label className="text-xs text-zinc-400 mb-1.5 block">Directory name</label>
                                <div className="flex gap-2">
                                    <input
                                        autoFocus
                                        value={dirValue}
                                        onChange={e => setDirValue(e.target.value)}
                                        onKeyDown={e => e.key === "Enter" && addDirectory()}
                                        placeholder="e.g. Season {season:02}"
                                        className="flex-1 bg-zinc-900 border border-zinc-600 rounded-md px-2.5 py-1.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:outline-none focus:border-amber-500/50"
                                    />
                                    <button onClick={addDirectory}
                                            className="px-3 py-1.5 bg-amber-500/20 text-amber-300 rounded-md text-sm font-medium hover:bg-amber-500/30 transition-colors">
                                        Add
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Add Separator */}
                    <div className="relative">
                        <button
                            onClick={() => {
                                setShowSepInput(!showSepInput);
                                setShowDirInput(false);
                            }}
                            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-slate-500/10 text-slate-300 border border-slate-500/30 text-sm font-medium hover:bg-slate-500/20 transition-colors"
                        >
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                                 strokeWidth="2" strokeLinecap="round">
                                <path d="M18 6L6 18"/>
                            </svg>
                            Add Separator / Text
                        </button>
                        {showSepInput && (
                            <div
                                className="absolute top-full mt-2 left-0 z-10 bg-zinc-800 border border-zinc-700 rounded-lg p-3 shadow-xl min-w-70">
                                <label className="text-xs text-zinc-400 mb-1.5 block">Quick presets</label>
                                <div className="flex gap-1.5 mb-3">
                                    {SEPARATOR_PRESETS.map(s => (
                                        <button
                                            key={s}
                                            onClick={() => addSeparator(s)}
                                            className="px-3 py-1 bg-zinc-700 text-zinc-300 rounded text-sm font-mono hover:bg-zinc-600 transition-colors"
                                        >
                                            {s === " " ? "␣" : s}
                                        </button>
                                    ))}
                                </div>
                                <label className="text-xs text-zinc-400 mb-1.5 block">Custom text</label>
                                <div className="flex gap-2">
                                    <input
                                        autoFocus
                                        value={sepValue}
                                        onChange={e => setSepValue(e.target.value)}
                                        onKeyDown={e => e.key === "Enter" && addSeparator()}
                                        placeholder='e.g. S{season:02}E{episode:02}'
                                        className="flex-1 bg-zinc-900 border border-zinc-600 rounded-md px-2.5 py-1.5 text-sm text-zinc-100 font-mono placeholder:text-zinc-600 focus:outline-none focus:border-slate-500/50"
                                    />
                                    <button onClick={() => addSeparator()}
                                            className="px-3 py-1.5 bg-slate-500/20 text-slate-300 rounded-md text-sm font-medium hover:bg-slate-500/30 transition-colors">
                                        Add
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                </div>

                {/* Preview */}
                <div className="space-y-2.5">
                    <label className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Preview</label>
                    <div className="bg-zinc-900/80 border border-zinc-800 rounded-lg px-4 py-3">
                        <code className="text-sm text-emerald-400 break-all">{getTemplateString() || "—"}</code>
                    </div>
                    <div className="bg-zinc-900/50 border border-zinc-800/50 rounded-lg px-4 py-3">
                        <span className="text-xs text-zinc-500 mr-2">Example:</span>
                        <code className="text-sm text-zinc-400 break-all">
                            {getTemplateString()
                                    .replace(/\{series\}/g, "Frieren: Beyond Journey's End")
                                    .replace(/\{season:02\}/g, "01")
                                    .replace(/\{season\}/g, "1")
                                    .replace(/\{season_title\}/g, "Frieren: Beyond Journey's End - Season 01")
                                    .replace(/\{episode:02\}/g, "01")
                                    .replace(/\{episode\}/g, "1")
                                    .replace(/\{title\}/g, "The Journey's End")
                                    .replace(/\{quality\}/g, "1080p")
                                    .replace(/\{audio\}/g, "ja-JP")
                                    .replace(/\{year\}/g, "2023")
                                || "—"}
                        </code>
                    </div>
                </div>

                {/* Raw template string */}
                <div className="space-y-2.5">
                    <label className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Raw Template</label>
                    <div className="bg-zinc-900/80 border border-zinc-800 rounded-lg px-4 py-2.5">
                        <code
                            className="text-sm text-zinc-200 font-mono break-all">{getTemplateString() || "empty"}</code>
                    </div>
                </div>
            </div>
        </div>
    );
}
