// ==== FUN_0042f48c @ 0042f48c ====

void FUN_0042f48c(UINT param_1,ushort *param_2,HMENU param_3)

{
  BOOL BVar1;
  MENUITEMINFOW local_c8;
  tagMENUITEMINFOW local_78;
  
  if ((*param_2 >> 0xe & 1) != 0) {
    SetMenuDefaultItem(param_3,0xffffffff,0);
    *param_2 = *param_2 & 0xbfff;
  }
  FUN_0050daf0((ulonglong *)&local_c8,0,0x48);
  local_c8.cbSize = 0x48;
  local_c8.fMask = 1;
  if ((*param_2 >> 8 & 1) != 0) {
    local_c8.fState = local_c8.fState | 3;
  }
  if ((*param_2 >> 9 & 1) != 0) {
    local_c8.fState = local_c8.fState | 8;
  }
  if ((*param_2 >> 0xc & 1) != 0) {
    local_c8.fState = local_c8.fState | 0x1000;
  }
  local_c8.wID = param_1;
  if ((*param_2 >> 0xd & 1) != 0) {
    FUN_0050daf0((ulonglong *)&local_78,0,0x48);
    local_78.cbSize = 0x48;
    local_78.fMask = 0x10;
    local_78.wID = param_1;
    BVar1 = GetMenuItemInfoW(param_3,param_1,0,&local_78);
    if (BVar1 != 0) {
      local_c8.fMask = local_c8.fMask | 0x10;
      local_c8.fType = local_c8.fType | local_78.fType & 0xfffff7fb;
      local_c8.dwTypeData = *(LPWSTR *)(param_2 + 4);
    }
  }
  SetMenuItemInfoW(param_3,param_1,0,&local_c8);
  return;
}


