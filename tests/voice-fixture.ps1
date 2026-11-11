param([Parameter(Mandatory)][string]$OutputPath)
$ErrorActionPreference='Stop'
# Synthetic speech only; never records the developer's microphone.
Add-Type -AssemblyName System.Speech
$synth=New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    $synth.SetOutputToWaveFile($OutputPath)
    $synth.Speak('Please stop. I would like to practice English with you today. Can we talk about my favourite books?')
} finally { $synth.Dispose() }
