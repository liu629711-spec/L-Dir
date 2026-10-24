// ==== FUN_00470a88 @ 00470a88 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8 FUN_00470a88(longlong param_1)

{
  int iVar1;
  BOOL BVar2;
  longlong *plVar3;
  undefined8 uVar4;
  int *piVar5;
  longlong local_4f8;
  longlong local_4f0;
  longlong local_4e8 [2];
  tagOFNW local_4d8;
  uint local_438;
  ulonglong local_434 [65];
  WCHAR local_228 [2];
  ulonglong local_224 [65];
  undefined8 local_18;
  
  local_18 = DAT_00547f48;
  local_438 = 0;
  FUN_0050daf0(local_434,0,0x204);
  local_228[0] = L'\0';
  local_228[1] = L'\0';
  FUN_0050daf0(local_224,0,0x204);
  local_438 = local_438 & 0xffff0000;
  FUN_0050daf0((ulonglong *)&local_4d8,0,0x98);
  local_4d8.hwndOwner = *(HWND *)(param_1 + 8);
  local_4d8.nMaxFile = 0x208;
  local_4d8.lpstrFilter = L"Q-Dir File(*.qdr)";
  local_4d8.lpstrFile = (LPWSTR)&local_438;
  local_4d8.nMaxFileTitle = 0x208;
  local_4d8.lpstrFileTitle = local_228;
  local_4d8.lStructSize = 0x98;
  local_4d8.nFilterIndex = 0;
  local_4d8.lpstrInitialDir = (LPCWSTR)0x0;
  local_4d8.Flags = 0;
  BVar2 = GetOpenFileNameW(&local_4d8);
  if (BVar2 == 0) {
    uVar4 = 0;
  }
  else {
    FUN_00407e50(&local_4f0,local_228);
    FUN_00407e50(&local_4f8,(LPCWSTR)&local_438);
    plVar3 = FUN_0040329c(local_4e8,&local_4f8);
    FUN_0046cc14(param_1,plVar3,1,0);
    piVar5 = (int *)(local_4f8 + -0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar5);
      }
    }
    piVar5 = (int *)(local_4f0 + -0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar5);
      }
    }
    uVar4 = 1;
  }
  return uVar4;
}


