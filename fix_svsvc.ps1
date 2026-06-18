Start-Transcript -Path "C:\Users\Public\fix_svsvc_log.txt" -Force
# Reconstruct HKLM svsvc service key
if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc')) {
    New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Force | Out-Null
}
try {
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'DisplayName' -Value '@%SystemRoot%\System32\svsvc.dll,-100' -Type String -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'Description' -Value '@%SystemRoot%\System32\svsvc.dll,-101' -Type String -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'ErrorControl' -Value 1 -Type DWord -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'ImagePath' -Value '%SystemRoot%\system32\svchost.exe -k LocalSystemNetworkRestricted -p' -Type ExpandString -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'ObjectName' -Value 'LocalSystem' -Type String -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'ServiceSidType' -Value 1 -Type DWord -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'Start' -Value 3 -Type DWord -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc' -Name 'Type' -Value 32 -Type DWord -Force

    if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc\Parameters')) {
        New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc\Parameters' -Force | Out-Null
    }
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc\Parameters' -Name 'ServiceDll' -Value '%SystemRoot%\System32\svsvc.dll' -Type ExpandString -Force
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\svsvc\Parameters' -Name 'ServiceDllUnloadOnStop' -Value 1 -Type DWord -Force

    Start-Service -Name svsvc -ErrorAction SilentlyContinue
} catch {
    Write-Error $_
}
Stop-Transcript
