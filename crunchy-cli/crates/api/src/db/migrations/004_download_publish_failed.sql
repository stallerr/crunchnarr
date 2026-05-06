ALTER TABLE downloads ADD COLUMN source_url TEXT;

UPDATE downloads
SET output_path = CASE
    WHEN output_path IS NULL OR output_path = '' OR instr(output_path, '://') > 0 THEN output_path
    WHEN substr(output_path, 1, 1) = '/' THEN 'file://' || output_path
    ELSE 'file://' || output_path
END
WHERE output_path IS NOT NULL
  AND output_path != ''
  AND instr(output_path, '://') = 0;
