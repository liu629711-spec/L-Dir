// ==== FUN_00481604 @ 00481604 ====

undefined8 FUN_00481604(longlong param_1)

{
  int iVar1;
  longlong *plVar2;
  wchar_t *pwVar3;
  int *piVar4;
  wchar_t *local_res8;
  WCHAR *local_res10;
  longlong local_res18 [2];
  
  FUN_0040331c((longlong *)&local_res10,"::{031E4825-7B94-4DC3-B131-E946B44C8DD5}");
  FUN_004ef70c(param_1 + 0x300,(longlong *)&local_res8);
  if (*(int *)(local_res8 + -4) == 0) {
    plVar2 = FUN_004efce4(param_1 + 0x300,local_res18);
    FUN_00402154((longlong *)&local_res8,plVar2);
    piVar4 = (int *)(local_res18[0] + -0xc);
    if (piVar4 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar4;
      *piVar4 = *piVar4 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_res18[0] + -0xc));
      }
    }
    if ((((*(int *)(local_res8 + -4) < 0) ||
         (pwVar3 = wcsstr(local_res8,L"ftp:"), pwVar3 == (wchar_t *)0x0)) ||
        ((int)((longlong)pwVar3 - (longlong)local_res8 >> 1) != 0)) &&
       (((iVar1 = FUN_0050e5a0(local_res8,local_res10), iVar1 == 0 ||
         (*(int *)(local_res8 + -4) < 0)) ||
        ((pwVar3 = wcsstr(local_res8,local_res10), pwVar3 == (wchar_t *)0x0 ||
         ((int)((longlong)pwVar3 - (longlong)local_res8 >> 1) != 0)))))) {
      pwVar3 = local_res8 + -6;
      if (pwVar3 != (wchar_t *)PTR_DAT_00546008) {
        LOCK();
        iVar1 = *(int *)pwVar3;
        *(int *)pwVar3 = *(int *)pwVar3 + -1;
        UNLOCK();
        if (iVar1 + -1 < 1) {
          thunk_FUN_0050e3a0(local_res8 + -6);
        }
      }
      local_res10 = local_res10 + -6;
      if (local_res10 != (WCHAR *)PTR_DAT_00546008) {
        LOCK();
        iVar1 = *(int *)local_res10;
        *(int *)local_res10 = *(int *)local_res10 + -1;
        UNLOCK();
        if (iVar1 + -1 < 1) {
          thunk_FUN_0050e3a0(local_res10);
        }
      }
      return 0;
    }
  }
  pwVar3 = local_res8 + -6;
  if (pwVar3 != (wchar_t *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *(int *)pwVar3;
    *(int *)pwVar3 = *(int *)pwVar3 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(local_res8 + -6);
    }
  }
  local_res10 = local_res10 + -6;
  if (local_res10 != (WCHAR *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *(int *)local_res10;
    *(int *)local_res10 = *(int *)local_res10 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(local_res10);
    }
  }
  return 1;
}


