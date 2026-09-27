module FormatTests exposing (suite)

{-| Tests for `Format`.

These exist because the grouping rule is the kind of thing that looks right
and is wrong. ₹1,00,000 and ₹100,000 are both plausible-looking renderings of
the same integer, and only one of them reads as "one lakh" to the people this
product is for. The old Rust implementation had equivalent assertions against
icu4x; these replace them.

-}

import Expect
import Format
import Test exposing (Test, describe, test)


suite : Test
suite =
    describe "Format"
        [ describe "count, Indian grouping"
            [ test "leaves short numbers alone" <|
                \_ -> Format.count "en-IN" 0 |> Expect.equal "0"
            , test "leaves three digits alone" <|
                \_ -> Format.count "en-IN" 999 |> Expect.equal "999"
            , test "groups the first thousand normally" <|
                \_ -> Format.count "en-IN" 1000 |> Expect.equal "1,000"
            , test "one lakh is 1,00,000 and not 100,000" <|
                \_ -> Format.count "en-IN" 100000 |> Expect.equal "1,00,000"
            , test "one crore pairs all the way up" <|
                \_ -> Format.count "en-IN" 12345678 |> Expect.equal "1,23,45,678"
            , test "handles negatives" <|
                \_ -> Format.count "en-IN" -100000 |> Expect.equal "-1,00,000"
            ]
        , describe "count, thousands grouping"
            [ test "groups in threes" <|
                \_ -> Format.count "en-US" 1000 |> Expect.equal "1,000"
            , test "one hundred thousand stays 100,000" <|
                \_ -> Format.count "en-US" 100000 |> Expect.equal "100,000"
            , test "millions" <|
                \_ -> Format.count "en-US" 1234567 |> Expect.equal "1,234,567"
            , test "an unknown locale falls back to thousands" <|
                \_ -> Format.count "fr-FR" 100000 |> Expect.equal "100,000"
            ]
        , describe "money, from minor units"
            [ test "rupees and paise" <|
                \_ -> Format.money "en-IN" "INR" 250000 |> Expect.equal "₹2,500.00"
            , test "pads a single paise digit" <|
                \_ -> Format.money "en-IN" "INR" 205 |> Expect.equal "₹2.05"
            , test "sub-rupee amounts keep a leading zero" <|
                \_ -> Format.money "en-IN" "INR" 5 |> Expect.equal "₹0.05"
            , test "one lakh rupees uses lakh grouping" <|
                \_ -> Format.money "en-IN" "INR" 10000000 |> Expect.equal "₹1,00,000.00"
            , test "dollars and cents" <|
                \_ -> Format.money "en-US" "USD" 2000000 |> Expect.equal "$20,000.00"
            , test "two cents" <|
                \_ -> Format.money "en-US" "USD" 2 |> Expect.equal "$0.02"
            , test "an unknown currency shows its code" <|
                \_ -> Format.money "en-US" "EUR" 100 |> Expect.equal "EUR 1.00"
            ]
        , describe "milliRate, from milli-minor units"
            [ test "ten paise per reply" <|
                -- The stored value for ₹0.10: 10 paise x 1000.
                \_ -> Format.milliRate "en-IN" "INR" 10000 |> Expect.equal "₹0.10"
            , test "a tenth of a cent keeps its third decimal" <|
                \_ -> Format.milliRate "en-US" "USD" 100 |> Expect.equal "$0.001"
            , test "never trims below two decimals" <|
                \_ -> Format.milliRate "en-IN" "INR" 100000 |> Expect.equal "₹1.00"
            , test "an unpriced currency reads as zero rather than blank" <|
                \_ -> Format.milliRate "en-IN" "INR" 0 |> Expect.equal "₹0.00"
            ]
        , describe "symbol"
            [ test "INR" <| \_ -> Format.symbol "INR" |> Expect.equal "₹"
            , test "USD" <| \_ -> Format.symbol "USD" |> Expect.equal "$"
            , test "is case-insensitive" <| \_ -> Format.symbol "inr" |> Expect.equal "₹"
            ]
        ]
