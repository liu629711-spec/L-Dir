// ==== FUN_004764a0 @ 004764a0 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void FUN_004764a0(int param_1,longlong *param_2)

{
  int iVar1;
  BOOL BVar2;
  int *piVar3;
  longlong local_2b8 [2];
  SHELLEXECUTEINFOW local_2a8;
  WCHAR local_238 [264];
  undefined8 local_28;
  
  local_28 = DAT_00547f48;
  GetModuleFileNameW(DAT_005556d0,local_238,0x104);
  FUN_0050daf0((ulonglong *)&local_2a8,0,0x70);
  local_2a8.cbSize = 0x70;
  FUN_00407e50(local_2b8,local_238);
  local_2a8.fMask = 0;
  local_2a8.lpFile = (LPCWSTR)FUN_00402340(local_2b8,0);
  local_2a8.lpParameters = (LPCWSTR)*param_2;
  local_2a8.lpClass = (LPCWSTR)0x0;
  local_2a8.hwnd = (HWND)0x0;
  local_2a8.lpVerb = (LPCWSTR)(-(ulonglong)(param_1 != 0) & 0x528100);
  local_2a8.nShow = 1;
  BVar2 = ShellExecuteExW(&local_2a8);
  if (BVar2 != 0) {
    PostQuitMessage(0xfefe);
  }
  piVar3 = (int *)(local_2b8[0] + -0xc);
  if (piVar3 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar3;
    *piVar3 = *piVar3 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar3);
    }
  }
  piVar3 = (int *)(*param_2 + -0xc);
  if (piVar3 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar3;
    *piVar3 = *piVar3 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0((LPVOID)(*param_2 + -0xc));
    }
  }
  return;
}


