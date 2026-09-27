module Format exposing (count, milliRate, money, symbol)

{-| Number and currency rendering.

This is pure Elm rather than a port to `Intl.NumberFormat`. Formatting happens
during `view`, and a port is asynchronous — the number would arrive a frame
after the markup that needs it. Two currencies and one grouping rule don't
justify that, and a pure function is testable.

The grouping rule that matters: Indian digit grouping is not thousands.
₹1,00,000 is one lakh — the last three digits group, then pairs. Getting this
wrong renders ₹100,000, which reads as a different number to the people this
product is for. See [`count`].

-}


{-| Group an integer for display in `locale`.

`en-IN` (and any `*-IN` tag) gets last-three-then-pairs; everything else gets
plain thousands.

    count "en-IN" 100000 --> "1,00,000"

    count "en-US" 100000 --> "100,000"

-}
count : String -> Int -> String
count locale n =
    let
        sign =
            if n < 0 then
                "-"

            else
                ""

        digits =
            String.fromInt (abs n)
    in
    sign
        ++ (if isIndian locale then
                groupIndian digits

            else
                groupThousands digits
           )


isIndian : String -> Bool
isIndian locale =
    String.endsWith "-IN" locale || locale == "hi" || locale == "ta"


{-| Last three digits, then groups of two.
-}
groupIndian : String -> String
groupIndian digits =
    if String.length digits <= 3 then
        digits

    else
        let
            lastThree =
                String.right 3 digits

            rest =
                String.dropRight 3 digits
        in
        joinReversedGroups 2 rest ++ "," ++ lastThree


{-| Plain groups of three.
-}
groupThousands : String -> String
groupThousands digits =
    joinReversedGroups 3 digits


{-| Split from the right into groups of `size` and join with commas.
-}
joinReversedGroups : Int -> String -> String
joinReversedGroups size digits =
    if String.length digits <= size then
        digits

    else
        joinReversedGroups size (String.dropRight size digits)
            ++ ","
            ++ String.right size digits


{-| Currency symbol for an ISO 4217 code.
-}
symbol : String -> String
symbol currency =
    case String.toUpper currency of
        "INR" ->
            "₹"

        "USD" ->
            "$"

        other ->
            other ++ " "


{-| Render an amount given in **minor units** (paise, cents).

    money "en-IN" "INR" 250000 --> "₹2,500.00"

-}
money : String -> String -> Int -> String
money locale currency amountMinor =
    let
        sign =
            if amountMinor < 0 then
                "-"

            else
                ""

        absolute =
            abs amountMinor

        major =
            absolute // 100

        minor =
            absolute |> modBy 100
    in
    sign
        ++ symbol currency
        ++ count locale major
        ++ "."
        ++ String.padLeft 2 '0' (String.fromInt minor)


{-| Render a per-reply rate given in **milli-minor** units (1/1000 of a paise
or cent), which is how the worker stores it so sub-paise prices fit.

₹0.10 per reply is stored as 10000. Trailing zeroes are trimmed so it reads
as ₹0.10 rather than ₹0.10000.

    milliRate "en-IN" "INR" 10000 --> "₹0.10"

    milliRate "en-US" "USD" 100 --> "$0.001"

-}
milliRate : String -> String -> Int -> String
milliRate locale currency milli =
    let
        -- Five decimal places of minor units: 2 for the minor unit itself,
        -- 3 for the milli fraction.
        major =
            milli // 100000

        remainder =
            milli |> modBy 100000

        decimals =
            String.padLeft 5 '0' (String.fromInt remainder)
                |> trimTrailingZeroes
    in
    symbol currency
        ++ count locale major
        ++ (if String.isEmpty decimals then
                ""

            else
                "." ++ decimals
           )


{-| Drop trailing zeroes, but never below two places: prices read as ₹0.10,
not ₹0.1.
-}
trimTrailingZeroes : String -> String
trimTrailingZeroes s =
    if String.length s > 2 && String.endsWith "0" s then
        trimTrailingZeroes (String.dropRight 1 s)

    else
        s
