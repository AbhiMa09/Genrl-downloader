# Generator for bundled community season mapping seed data.
# Emits seed JSON with per-show source URL and retrieval date.

$ErrorActionPreference = "Stop"
$RetrievalDate = "2026-11-09"

$Seeds = @(
    @{
        mal_id = 235
        title = "Meitantei Conan"
        source_url = "https://en.wikipedia.org/wiki/List_of_Case_Closed_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 28; title = "Season 1"; year = 1996 },
            @{ season_no = 2; first_ep = 29; last_ep = 54; title = "Season 2"; year = 1996 },
            @{ season_no = 3; first_ep = 55; last_ep = 82; title = "Season 3"; year = 1997 },
            @{ season_no = 4; first_ep = 83; last_ep = 106; title = "Season 4"; year = 1997 },
            @{ season_no = 5; first_ep = 107; last_ep = 134; title = "Season 5"; year = 1998 },
            @{ season_no = 6; first_ep = 135; last_ep = 162; title = "Season 6"; year = 1999 },
            @{ season_no = 7; first_ep = 163; last_ep = 193; title = "Season 7"; year = 1999 },
            @{ season_no = 8; first_ep = 194; last_ep = 219; title = "Season 8"; year = 2000 },
            @{ season_no = 9; first_ep = 220; last_ep = 254; title = "Season 9"; year = 2001 },
            @{ season_no = 10; first_ep = 255; last_ep = 285; title = "Season 10"; year = 2001 },
            @{ season_no = 11; first_ep = 286; last_ep = 315; title = "Season 11"; year = 2002 },
            @{ season_no = 12; first_ep = 316; last_ep = 353; title = "Season 12"; year = 2003 },
            @{ season_no = 13; first_ep = 354; last_ep = 389; title = "Season 13"; year = 2004 },
            @{ season_no = 14; first_ep = 390; last_ep = 426; title = "Season 14"; year = 2005 },
            @{ season_no = 15; first_ep = 427; last_ep = 465; title = "Season 15"; year = 2006 },
            @{ season_no = 16; first_ep = 466; last_ep = 490; title = "Season 16"; year = 2007 },
            @{ season_no = 17; first_ep = 491; last_ep = 523; title = "Season 17"; year = 2008 },
            @{ season_no = 18; first_ep = 524; last_ep = 565; title = "Season 18"; year = 2009 },
            @{ season_no = 19; first_ep = 566; last_ep = 605; title = "Season 19"; year = 2010 },
            @{ season_no = 20; first_ep = 606; last_ep = 645; title = "Season 20"; year = 2011 },
            @{ season_no = 21; first_ep = 646; last_ep = 680; title = "Season 21"; year = 2012 },
            @{ season_no = 22; first_ep = 681; last_ep = 718; title = "Season 22"; year = 2013 },
            @{ season_no = 23; first_ep = 719; last_ep = 739; title = "Season 23"; year = 2013 },
            @{ season_no = 24; first_ep = 740; last_ep = 778; title = "Season 24"; year = 2014 },
            @{ season_no = 25; first_ep = 779; last_ep = 817; title = "Season 25"; year = 2015 },
            @{ season_no = 26; first_ep = 818; last_ep = 856; title = "Season 26"; year = 2016 },
            @{ season_no = 27; first_ep = 857; last_ep = 897; title = "Season 27"; year = 2017 },
            @{ season_no = 28; first_ep = 898; last_ep = 926; title = "Season 28"; year = 2018 },
            @{ season_no = 29; first_ep = 927; last_ep = 959; title = "Season 29"; year = 2019 },
            @{ season_no = 30; first_ep = 960; last_ep = 999; title = "Season 30"; year = 2019 },
            @{ season_no = 31; first_ep = 1000; last_ep = 1039; title = "Season 31"; year = 2021 },
            @{ season_no = 32; first_ep = 1040; last_ep = 1049; title = "Season 32"; year = 2022 },
            @{ season_no = 33; first_ep = 1050; last_ep = 1093; title = "Season 33"; year = 2023 }
        )
    },
    @{
        mal_id = 21
        title = "One Piece"
        source_url = "https://en.wikipedia.org/wiki/Lists_of_One_Piece_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 61; title = "East Blue"; year = 1999 },
            @{ season_no = 2; first_ep = 62; last_ep = 77; title = "Entering into the Grand Line"; year = 2001 },
            @{ season_no = 3; first_ep = 78; last_ep = 92; title = "Drum Island"; year = 2001 },
            @{ season_no = 4; first_ep = 93; last_ep = 130; title = "Alabasta"; year = 2001 },
            @{ season_no = 5; first_ep = 131; last_ep = 143; title = "TV Original"; year = 2002 },
            @{ season_no = 6; first_ep = 144; last_ep = 195; title = "Skypiea"; year = 2003 },
            @{ season_no = 7; first_ep = 196; last_ep = 228; title = "Escape! The Naval Fortress & The Foxy Pirate Crew"; year = 2004 },
            @{ season_no = 8; first_ep = 229; last_ep = 263; title = "Water Seven"; year = 2005 },
            @{ season_no = 9; first_ep = 264; last_ep = 336; title = "Enies Lobby"; year = 2006 },
            @{ season_no = 10; first_ep = 337; last_ep = 381; title = "Thriller Bark"; year = 2008 },
            @{ season_no = 11; first_ep = 382; last_ep = 407; title = "Sabaody Archipelago"; year = 2008 },
            @{ season_no = 12; first_ep = 408; last_ep = 421; title = "Amazon Lily"; year = 2009 },
            @{ season_no = 13; first_ep = 422; last_ep = 456; title = "Impel Down"; year = 2009 },
            @{ season_no = 14; first_ep = 457; last_ep = 516; title = "Marineford"; year = 2010 },
            @{ season_no = 15; first_ep = 517; last_ep = 578; title = "Fish-Man Island"; year = 2011 },
            @{ season_no = 16; first_ep = 579; last_ep = 628; title = "Punk Hazard"; year = 2013 },
            @{ season_no = 17; first_ep = 629; last_ep = 746; title = "Dressrosa"; year = 2014 },
            @{ season_no = 18; first_ep = 747; last_ep = 782; title = "Zou"; year = 2016 },
            @{ season_no = 19; first_ep = 783; last_ep = 891; title = "Whole Cake Island"; year = 2017 },
            @{ season_no = 20; first_ep = 892; last_ep = 1088; title = "Wano Country"; year = 2019 },
            @{ season_no = 21; first_ep = 1089; last_ep = 1155; title = "Egghead"; year = 2024 }
        )
    },
    @{
        mal_id = 813
        title = "Dragon Ball Z"
        source_url = "https://en.wikipedia.org/wiki/List_of_Dragon_Ball_Z_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 39; title = "Saiyan Saga"; year = 1989 },
            @{ season_no = 2; first_ep = 40; last_ep = 74; title = "Namek and Captain Ginyu Sagas"; year = 1990 },
            @{ season_no = 3; first_ep = 75; last_ep = 107; title = "Frieza Saga"; year = 1991 },
            @{ season_no = 4; first_ep = 108; last_ep = 139; title = "Garlic Jr., Trunks and Androids Sagas"; year = 1991 },
            @{ season_no = 5; first_ep = 140; last_ep = 165; title = "Imperfect and Perfect Cell Sagas"; year = 1992 },
            @{ season_no = 6; first_ep = 166; last_ep = 194; title = "Cell Games Saga"; year = 1992 },
            @{ season_no = 7; first_ep = 195; last_ep = 219; title = "World Tournament Saga"; year = 1993 },
            @{ season_no = 8; first_ep = 220; last_ep = 253; title = "Babidi and Majin Buu Sagas"; year = 1994 },
            @{ season_no = 9; first_ep = 254; last_ep = 291; title = "Fusion and Kid Buu Sagas"; year = 1995 }
        )
    },
    @{
        mal_id = 20
        title = "Naruto"
        source_url = "https://en.wikipedia.org/wiki/List_of_Naruto_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 35; title = "Season 1"; year = 2002 },
            @{ season_no = 2; first_ep = 36; last_ep = 83; title = "Season 2"; year = 2003 },
            @{ season_no = 3; first_ep = 84; last_ep = 131; title = "Season 3"; year = 2004 },
            @{ season_no = 4; first_ep = 132; last_ep = 179; title = "Season 4"; year = 2005 },
            @{ season_no = 5; first_ep = 180; last_ep = 220; title = "Season 5"; year = 2006 }
        )
    },
    @{
        mal_id = 1735
        title = "Naruto: Shippuuden"
        source_url = "https://en.wikipedia.org/wiki/List_of_Naruto:_Shippuden_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 32; title = "Season 1"; year = 2007 },
            @{ season_no = 2; first_ep = 33; last_ep = 53; title = "Season 2"; year = 2007 },
            @{ season_no = 3; first_ep = 54; last_ep = 71; title = "Season 3"; year = 2008 },
            @{ season_no = 4; first_ep = 72; last_ep = 88; title = "Season 4"; year = 2008 },
            @{ season_no = 5; first_ep = 89; last_ep = 112; title = "Season 5"; year = 2008 },
            @{ season_no = 6; first_ep = 113; last_ep = 143; title = "Season 6"; year = 2009 },
            @{ season_no = 7; first_ep = 144; last_ep = 151; title = "Season 7"; year = 2010 },
            @{ season_no = 8; first_ep = 152; last_ep = 175; title = "Season 8"; year = 2010 },
            @{ season_no = 9; first_ep = 176; last_ep = 196; title = "Season 9"; year = 2010 },
            @{ season_no = 10; first_ep = 197; last_ep = 221; title = "Season 10"; year = 2011 },
            @{ season_no = 11; first_ep = 222; last_ep = 242; title = "Season 11"; year = 2011 },
            @{ season_no = 12; first_ep = 243; last_ep = 275; title = "Season 12"; year = 2012 },
            @{ season_no = 13; first_ep = 276; last_ep = 295; title = "Season 13"; year = 2012 },
            @{ season_no = 14; first_ep = 296; last_ep = 320; title = "Season 14"; year = 2013 },
            @{ season_no = 15; first_ep = 321; last_ep = 348; title = "Season 15"; year = 2013 },
            @{ season_no = 16; first_ep = 349; last_ep = 361; title = "Season 16"; year = 2014 },
            @{ season_no = 17; first_ep = 362; last_ep = 372; title = "Season 17"; year = 2014 },
            @{ season_no = 18; first_ep = 373; last_ep = 393; title = "Season 18"; year = 2014 },
            @{ season_no = 19; first_ep = 394; last_ep = 413; title = "Season 19"; year = 2015 },
            @{ season_no = 20; first_ep = 414; last_ep = 458; title = "Season 20"; year = 2015 },
            @{ season_no = 21; first_ep = 459; last_ep = 479; title = "Season 21"; year = 2016 },
            @{ season_no = 22; first_ep = 480; last_ep = 500; title = "Season 22"; year = 2016 }
        )
    },
    @{
        mal_id = 269
        title = "Bleach"
        source_url = "https://en.wikipedia.org/wiki/List_of_Bleach_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 20; title = "The Substitute"; year = 2004 },
            @{ season_no = 2; first_ep = 21; last_ep = 41; title = "The Entry"; year = 2005 },
            @{ season_no = 3; first_ep = 42; last_ep = 63; title = "The Rescue"; year = 2005 },
            @{ season_no = 4; first_ep = 64; last_ep = 91; title = "The Bount"; year = 2006 },
            @{ season_no = 5; first_ep = 92; last_ep = 109; title = "The Assault"; year = 2006 },
            @{ season_no = 6; first_ep = 110; last_ep = 131; title = "Arrancar: The Appearance"; year = 2007 },
            @{ season_no = 7; first_ep = 132; last_ep = 151; title = "The Hueco Mundo Sneak Entry"; year = 2007 },
            @{ season_no = 8; first_ep = 152; last_ep = 167; title = "The Fierce Fight"; year = 2007 },
            @{ season_no = 9; first_ep = 168; last_ep = 189; title = "The New Captain Shusuke Amagai"; year = 2008 },
            @{ season_no = 10; first_ep = 190; last_ep = 205; title = "Arrancar vs. Soul Reaper"; year = 2008 },
            @{ season_no = 11; first_ep = 206; last_ep = 212; title = "Turn Back the Pendulum"; year = 2009 },
            @{ season_no = 12; first_ep = 213; last_ep = 229; title = "Battle in Karakura"; year = 2009 },
            @{ season_no = 13; first_ep = 230; last_ep = 265; title = "Zanpakuto: The Alternate Tale"; year = 2009 },
            @{ season_no = 14; first_ep = 266; last_ep = 316; title = "Fall of the Espada"; year = 2010 },
            @{ season_no = 15; first_ep = 317; last_ep = 342; title = "Gotei 13 Invading Army"; year = 2011 },
            @{ season_no = 16; first_ep = 343; last_ep = 366; title = "The Lost Agent"; year = 2011 }
        )
    },
    @{
        mal_id = 34572
        title = "Black Clover"
        source_url = "https://en.wikipedia.org/wiki/List_of_Black_Clover_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 51; title = "Season 1"; year = 2017 },
            @{ season_no = 2; first_ep = 52; last_ep = 102; title = "Season 2"; year = 2018 },
            @{ season_no = 3; first_ep = 103; last_ep = 154; title = "Season 3"; year = 2019 },
            @{ season_no = 4; first_ep = 155; last_ep = 170; title = "Season 4"; year = 2020 }
        )
    },
    @{
        mal_id = 6702
        title = "Fairy Tail"
        source_url = "https://en.wikipedia.org/wiki/List_of_Fairy_Tail_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 48; title = "Season 1"; year = 2009 },
            @{ season_no = 2; first_ep = 49; last_ep = 72; title = "Season 2"; year = 2010 },
            @{ season_no = 3; first_ep = 73; last_ep = 100; title = "Season 3"; year = 2011 },
            @{ season_no = 4; first_ep = 101; last_ep = 125; title = "Season 4"; year = 2011 },
            @{ season_no = 5; first_ep = 126; last_ep = 150; title = "Season 5"; year = 2012 },
            @{ season_no = 6; first_ep = 151; last_ep = 175; title = "Season 6"; year = 2012 }
        )
    },
    @{
        mal_id = 11061
        title = "Hunter x Hunter (2011)"
        source_url = "https://en.wikipedia.org/wiki/List_of_Hunter_%C3%97_Hunter_(2011_TV_series)_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 26; title = "Hunter Exam arc"; year = 2011 },
            @{ season_no = 2; first_ep = 27; last_ep = 38; title = "Zoldyck Family & Heavens Arena arcs"; year = 2012 },
            @{ season_no = 3; first_ep = 39; last_ep = 58; title = "Phantom Troupe arc"; year = 2012 },
            @{ season_no = 4; first_ep = 59; last_ep = 75; title = "Greed Island arc"; year = 2012 },
            @{ season_no = 5; first_ep = 76; last_ep = 136; title = "Chimera Ant arc"; year = 2013 },
            @{ season_no = 6; first_ep = 137; last_ep = 148; title = "Election arc"; year = 2014 }
        )
    },
    @{
        mal_id = 918
        title = "Gintama"
        source_url = "https://en.wikipedia.org/wiki/List_of_Gintama_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 49; title = "Season 1"; year = 2006 },
            @{ season_no = 2; first_ep = 50; last_ep = 99; title = "Season 2"; year = 2007 },
            @{ season_no = 3; first_ep = 100; last_ep = 150; title = "Season 3"; year = 2008 },
            @{ season_no = 4; first_ep = 151; last_ep = 201; title = "Season 4"; year = 2009 }
        )
    },
    @{
        mal_id = 249
        title = "InuYasha"
        source_url = "https://en.wikipedia.org/wiki/List_of_Inuyasha_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 27; title = "Season 1"; year = 2000 },
            @{ season_no = 2; first_ep = 28; last_ep = 54; title = "Season 2"; year = 2001 },
            @{ season_no = 3; first_ep = 55; last_ep = 82; title = "Season 3"; year = 2002 },
            @{ season_no = 4; first_ep = 83; last_ep = 110; title = "Season 4"; year = 2002 },
            @{ season_no = 5; first_ep = 111; last_ep = 138; title = "Season 5"; year = 2003 },
            @{ season_no = 6; first_ep = 139; last_ep = 162; title = "Season 6"; year = 2003 },
            @{ season_no = 7; first_ep = 163; last_ep = 167; title = "Season 7"; year = 2004 }
        )
    },
    @{
        mal_id = 223
        title = "Dragon Ball"
        source_url = "https://en.wikipedia.org/wiki/List_of_Dragon_Ball_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 31; title = "Season 1"; year = 1986 },
            @{ season_no = 2; first_ep = 32; last_ep = 57; title = "Season 2"; year = 1986 },
            @{ season_no = 3; first_ep = 58; last_ep = 83; title = "Season 3"; year = 1987 },
            @{ season_no = 4; first_ep = 84; last_ep = 101; title = "Season 4"; year = 1987 },
            @{ season_no = 5; first_ep = 102; last_ep = 122; title = "Season 5"; year = 1988 },
            @{ season_no = 6; first_ep = 123; last_ep = 132; title = "Season 6"; year = 1988 },
            @{ season_no = 7; first_ep = 133; last_ep = 140; title = "Season 7"; year = 1988 },
            @{ season_no = 8; first_ep = 141; last_ep = 148; title = "Season 8"; year = 1989 },
            @{ season_no = 9; first_ep = 149; last_ep = 153; title = "Season 9"; year = 1989 }
        )
    },
    @{
        mal_id = 392
        title = "Yuu☆Yuu☆Hakusho"
        source_url = "https://en.wikipedia.org/wiki/List_of_YuYu_Hakusho_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 25; title = "Spirit Detective Saga"; year = 1992 },
            @{ season_no = 2; first_ep = 26; last_ep = 66; title = "Dark Tournament Saga"; year = 1993 },
            @{ season_no = 3; first_ep = 67; last_ep = 94; title = "Chapter Black Saga"; year = 1994 },
            @{ season_no = 4; first_ep = 95; last_ep = 112; title = "Three Kings Saga"; year = 1994 }
        )
    },
    @{
        mal_id = 5114
        title = "Fullmetal Alchemist: Brotherhood"
        source_url = "https://en.wikipedia.org/wiki/List_of_Fullmetal_Alchemist:_Brotherhood_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 14; title = "Season 1"; year = 2009 },
            @{ season_no = 2; first_ep = 15; last_ep = 26; title = "Season 2"; year = 2009 },
            @{ season_no = 3; first_ep = 27; last_ep = 39; title = "Season 3"; year = 2009 },
            @{ season_no = 4; first_ep = 40; last_ep = 51; title = "Season 4"; year = 2010 },
            @{ season_no = 5; first_ep = 52; last_ep = 64; title = "Season 5"; year = 2010 }
        )
    },
    @{
        mal_id = 1604
        title = "Katekyou Hitman Reborn!"
        source_url = "https://en.wikipedia.org/wiki/List_of_Reborn!_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 33; title = "Season 1"; year = 2006 },
            @{ season_no = 2; first_ep = 34; last_ep = 65; title = "Vs. Varia"; year = 2007 },
            @{ season_no = 3; first_ep = 66; last_ep = 73; title = "Daily Life"; year = 2008 },
            @{ season_no = 4; first_ep = 74; last_ep = 101; title = "The Future"; year = 2008 },
            @{ season_no = 5; first_ep = 102; last_ep = 141; title = "The Future X"; year = 2008 },
            @{ season_no = 6; first_ep = 142; last_ep = 153; title = "The Arcobaleno"; year = 2009 },
            @{ season_no = 7; first_ep = 154; last_ep = 177; title = "The Choice"; year = 2009 },
            @{ season_no = 8; first_ep = 178; last_ep = 189; title = "The Primo Family"; year = 2010 },
            @{ season_no = 9; first_ep = 190; last_ep = 203; title = "The Final Future"; year = 2010 }
        )
    },
    @{
        mal_id = 225
        title = "Dragon Ball GT"
        source_url = "https://en.wikipedia.org/wiki/List_of_Dragon_Ball_GT_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 31; title = "Season 1"; year = 1996 },
            @{ season_no = 2; first_ep = 32; last_ep = 64; title = "Season 2"; year = 1996 }
        )
    },
    @{
        mal_id = 232
        title = "Cardcaptor Sakura"
        source_url = "https://en.wikipedia.org/wiki/List_of_Cardcaptor_Sakura_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 35; title = "Season 1"; year = 1998 },
            @{ season_no = 2; first_ep = 36; last_ep = 46; title = "Season 2"; year = 1999 },
            @{ season_no = 3; first_ep = 47; last_ep = 70; title = "Season 3"; year = 1999 }
        )
    },
    @{
        mal_id = 45
        title = "Rurouni Kenshin: Meiji Kenkaku Romantan"
        source_url = "https://en.wikipedia.org/wiki/List_of_Rurouni_Kenshin_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 27; title = "Season 1"; year = 1996 },
            @{ season_no = 2; first_ep = 28; last_ep = 62; title = "Kyoto Arc"; year = 1996 },
            @{ season_no = 3; first_ep = 63; last_ep = 95; title = "Season 3"; year = 1997 }
        )
    },
    @{
        mal_id = 1482
        title = "D.Gray-man"
        source_url = "https://en.wikipedia.org/wiki/List_of_D.Gray-man_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 51; title = "Season 1"; year = 2006 },
            @{ season_no = 2; first_ep = 52; last_ep = 103; title = "Season 2"; year = 2007 }
        )
    },
    @{
        mal_id = 19
        title = "Monster"
        source_url = "https://en.wikipedia.org/wiki/List_of_Monster_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 37; title = "Season 1"; year = 2004 },
            @{ season_no = 2; first_ep = 38; last_ep = 74; title = "Season 2"; year = 2004 }
        )
    },
    @{
        mal_id = 1535
        title = "Death Note"
        source_url = "https://en.wikipedia.org/wiki/List_of_Death_Note_episodes"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 19; title = "Season 1"; year = 2006 },
            @{ season_no = 2; first_ep = 20; last_ep = 37; title = "Season 2"; year = 2007 }
        )
    },
    @{
        mal_id = 52991
        title = "Sousou no Frieren"
        source_url = "https://en.wikipedia.org/wiki/Frieren"
        retrieval_date = $RetrievalDate
        seasons = @(
            @{ season_no = 1; first_ep = 1; last_ep = 28; title = "Season 1"; year = 2023 }
        )
    }
)

$OutDir = Join-Path $PSScriptRoot "..\core\crates\genrl-core\src\db\seeds"
if (!(Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}
$OutFile = Join-Path $OutDir "season_map.json"
$Json = $Seeds | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText($OutFile, $Json, [System.Text.Encoding]::UTF8)

Write-Host "Generated season map seeds: $($Seeds.Count) shows -> $OutFile"
