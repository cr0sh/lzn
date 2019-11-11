DELETE FROM titles
WHERE  provider = 'lezhin'
       AND EXISTS(SELECT comic
                  FROM   lezhin
                  WHERE  comic = titles.id)  