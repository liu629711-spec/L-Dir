// ==== FUN_0045a1e0 @ 0045a1e0 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

HINSTANCE FUN_0045a1e0(longlong param_1,longlong *param_2)

{
  int iVar1;
  undefined *puVar2;
  HINSTANCE pHVar3;
  int *piVar4;
  wchar_t *lpFile;
  LPCWSTR lpParameters;
  INT nShowCmd;
  LPCWSTR local_258 [2];
  WCHAR local_248 [264];
  undefined8 local_38;
  
  puVar2 = PTR_DAT_00546000;
  local_38 = DAT_00547f48;
  GetModuleFileNameW(DAT_005556d0,local_248,0x104);
  FUN_00407e50((longlong *)local_258,local_248);
  if (DAT_00550fd0 == 2) {
    DAT_00550fd0 = 0;
    nShowCmd = 5;
    lpFile = L"explorer.exe";
    lpParameters = local_258[0];
  }
  else {
    lpParameters = (LPCWSTR)*param_2;
    nShowCmd = 1;
    lpFile = local_258[0];
  }
  pHVar3 = ShellExecuteW(*(HWND *)(param_1 + 8),L"open",lpFile,lpParameters,(LPCWSTR)0x0,nShowCmd);
  local_258[0] = local_258[0] + -6;
  if (local_258[0] != (LPCWSTR)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *(int *)local_258[0];
    *(int *)local_258[0] = *(int *)local_258[0] + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(local_258[0]);
    }
  }
  piVar4 = (int *)(puVar2 + -0xc);
  if (piVar4 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar4;
    *piVar4 = *piVar4 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar4);
    }
  }
  piVar4 = (int *)(*param_2 + -0xc);
  if (piVar4 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar4;
    *piVar4 = *piVar4 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0((LPVOID)(*param_2 + -0xc));
    }
  }
  return pHVar3;
}


