#[rustfmt::skip]
pub const USAGE_MESSAGE : &str = "
solana-dekey is a utility program that replaces Solana validator identity
pubkeys and vote account pubkeys with the validator's name.

Unless the -l (--lookup) option is provided, it reads its standard input,
replaces keys with names, and writes the result to its standard output.

If the -l (--lookup) option is provided, instead it takes as a single
argument a regular expression and searches for validators whose names match
that regular expression, and for those found prints out the validator
identity and vote account pubkeys.

Arguments:

  -u (--url) -- Specifies the RPC URL to use for querying for validator
                  information.  Defaults to \"mainnet\".  The following
                  special values can be used, which map as follows:
                     l (or localhost) -- http://localhost:8899
                     d (or devnet)    -- https://api.devnet.solana.com
                     t (or testnet)   -- https://api.testnet.solana.com
                     m (or mainnet)   -- https://api.mainnet-beta.solana.com

  -c (--cache_file) -- Specifies the full path to the cache file in which
                         to store the key to name mappings.  Defaults to
                         ~/.solana-dekey-cache

  -d (--delete_cache) -- Delete (and reload from RPC) the validator key
                           to name map before proceeding

  -l (--lookup) -- Instructs solana-dekey to look up validator keys by
                     validator name instead of replacing keys with names

  -i (--identity) -- Do mapping only from validator identity to name, not
                       vote account to name.  Defaults to false.

  -v (--vote) -- Do mapping only from vote account to name, not validator
                   identity to name.  Defaults to false.

  -a (--ascii) -- Strip all non-ASCII characters from validator names
                    before substituting them.

  -f (--fit) -- Pad (with spaces) the validator names so that they have
                  the same length as the key they are replacing.  This
                  is useful when the input being processed is in tabular
                  form, because it retains the spacing of the table
                  columns.
";
