
``` sql
SELECT e.pos,
       se.entry_rank,se.synset_rank,
       f.form,
       STRING_AGG(m.form, ', ') AS synset_members,
       d.definition
  FROM entries e
  JOIN forms f ON f.entry_rowid = e.rowid
  JOIN senses se ON se.entry_rowid = e.rowid
  JOIN synsets s ON se.synset_rowid = s.rowid
  JOIN definitions d ON d.synset_rowid = s.rowid
  JOIN senses se2 ON se2.synset_rowid = s.rowid
  JOIN forms m ON m.entry_rowid = se2.entry_rowid AND m.rank = 0
  WHERE f.form = 'bank'
  GROUP BY se.rowid, e.pos, se.entry_rank, se.synset_rank, f.form, d.definition
  ORDER BY e.pos, se.entry_rank;
```

``` sql
SELECT e.id, e.pos, f.form, f.rank,
       p.value AS ipa, p.variety, p.notation, p.phonemic, p.audio
  FROM entries e
  JOIN forms f ON f.entry_rowid = e.rowid
  LEFT JOIN pronunciations p ON p.form_rowid = f.rowid
  WHERE f.form = 'bank'
  ORDER BY f.rank, p.variety;
```

``` sql
SELECT e.pos,
         se.entry_rank,
         se.synset_rank,
         f.form,
         STRING_AGG(DISTINCT m.form, ', ' ORDER BY m.form) AS synset_members,
         d.definition,
         STRING_AGG(DISTINCT f_cmn.form, '；') FILTER (WHERE f_cmn.form IS NOT NULL) AS zh_members
    FROM entries e
    JOIN forms f ON f.entry_rowid = e.rowid
    JOIN senses se ON se.entry_rowid = e.rowid
    JOIN synsets s ON se.synset_rowid = s.rowid
    JOIN definitions d ON d.synset_rowid = s.rowid
    JOIN senses se2 ON se2.synset_rowid = s.rowid
    JOIN forms m ON m.entry_rowid = se2.entry_rowid AND m.rank = 0
    LEFT JOIN ilis i ON s.ili_rowid = i.rowid
    LEFT JOIN synsets s_cmn ON s_cmn.ili_rowid = i.rowid AND s_cmn.lexicon_rowid = 2
    LEFT JOIN senses se_cmn ON se_cmn.synset_rowid = s_cmn.rowid
    LEFT JOIN forms f_cmn ON f_cmn.entry_rowid = se_cmn.entry_rowid AND f_cmn.rank = 0
    WHERE f.form = 'bank'
    GROUP BY se.rowid, e.pos, se.entry_rank, se.synset_rank, f.form, d.definition
    ORDER BY e.pos, se.entry_rank;
```