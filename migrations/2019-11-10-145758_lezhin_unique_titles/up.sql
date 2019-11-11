INSERT INTO titles
            (provider,
             id,
             title)
SELECT 'lezhin',
       lezhin.comic,
       NULL
FROM   lezhin
GROUP  BY lezhin.comic